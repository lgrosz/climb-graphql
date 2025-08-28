use async_graphql::{Context, InputObject, Object, Result, SimpleObject, Union, ID};

use crate::schema::Image;
use crate::AppData;

use super::climb::Climb;
use super::crag::Crag;
use super::region::Region;
use super::sector::Sector;

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "CoordinateInput")]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Union)]
enum FormationParent {
    Region(Region),
    Crag(Crag),
    Sector(Sector),
}

pub struct Formation(pub i32);

#[Object]
impl Formation {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn name(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT name FROM climb.formations WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn description(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT description FROM climb.formations WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|description| description.to_string()))
    }

    async fn location(&self, ctx: &Context<'_>) -> Result<Option<Coordinate>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let maybe_point = client
            .query_one("SELECT location FROM climb.formations WHERE id = $1", &[&self.0])
            .await?
            .try_get::<_, Option<postgis::ewkb::Point>>(0)?;

        Ok(maybe_point.map(|point| Coordinate {
            latitude: point.y,
            longitude: point.x,
        }))
    }

    async fn parent(&self, ctx: &Context<'_>) -> Result<Option<FormationParent>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => return Err("Database connection is not available".into()),
        };

        let row = client
            .query_one(
                "
                SELECT region_id, crag_id, sector_id
                FROM climb.formations
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let region_id: Option<i32> = row.try_get(0)?;
        let crag_id: Option<i32> = row.try_get(1)?;
        let sector_id: Option<i32> = row.try_get(2)?;

        let parent = match (region_id, crag_id, sector_id) {
            (Some(id), None, None) => Some(FormationParent::Region(Region(id))),
            (None, Some(id), None) => Some(FormationParent::Crag(Crag(id))),
            (None, None, Some(id)) => Some(FormationParent::Sector(Sector(id))),
            (None, None, None) => None,
            _ => return Err("Formation has multiple parents, which violates schema".into()),
        };

        Ok(parent)
    }

    async fn climbs(&self, ctx: &Context<'_>) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT id FROM climb.climbs WHERE formation_id = $1", &[&self.0])
            .await?;

        let climbs = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climb))
            .collect::<Result<Vec<Climb>, _>>()?;

        Ok(climbs)
    }

    async fn images(&self, ctx: &Context<'_>) -> Result<Vec<Image>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT image_id FROM climb.formations_in_image WHERE formation_id = $1", &[&self.0])
            .await?;

        let images = result
            .into_iter()
            .map(|row| row.try_get(0).map(Image))
            .collect::<Result<Vec<Image>, _>>()?;

        Ok(images)
    }
}
