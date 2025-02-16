use async_graphql::{Context, InputObject, Object, Result, SimpleObject, Union, ID};

use crate::schema::area::Area;
use crate::schema::climb::Climb;
use crate::AppData;

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "CoordinateInput")]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Union)]
enum FormationParent {
    Area(Area),
    Formation(Formation),
}

pub struct Formation(pub i32);

#[Object]
impl Formation {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT name FROM formations WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn description<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT description FROM formations WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|description| description.to_string()))
    }

    async fn location<'a>(&self, ctx: &Context<'a>) -> Result<Option<Coordinate>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let maybe_point = client
            .query_one("SELECT location FROM formations WHERE id = $1", &[&self.0])
            .await?
            .try_get::<_, Option<postgis::ewkb::Point>>(0)?;

        Ok(maybe_point.map(|point| Coordinate {
            latitude: point.y,
            longitude: point.x,
        }))
    }

    async fn parent<'a>(&self, ctx: &Context<'a>) -> Result<Option<FormationParent>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_opt(
                // TODO The JOIN is unecessary since we know only one can exist given the checks.
                // Is there a more effecient method?
                "
                SELECT sac.super_area_id, sfc.super_formation_id
                FROM formation_super_area_closures sac
                FULL JOIN formation_super_formation_closures sfc 
                ON sac.formation_id = sfc.formation_id
                WHERE sac.formation_id = $1 OR sfc.formation_id = $1
                ",
                &[&self.0],
            )
            .await?;

        if let Some(row) = result {
            match (
                row.try_get::<_, Option<i32>>(0)?,
                row.try_get::<_, Option<i32>>(1)?,
            ) {
                (Some(super_area_id), _) => {
                    return Ok(Some(FormationParent::Area(Area(super_area_id))))
                }
                (_, Some(super_formation_id)) => {
                    return Ok(Some(FormationParent::Formation(Formation(super_formation_id))))
                }
                _ => {}
            }
        }

        Ok(None)
    }

    async fn formations<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Formation>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client.query("SELECT formation_id FROM formation_super_formation_closures WHERE super_formation_id = $1", &[&self.0]).await?;
        let formations = result
            .into_iter()
            .map(|row| row.try_get(0).map(Formation))
            .collect::<Result<Vec<Formation>, _>>()?;

        Ok(formations)
    }

    async fn climbs<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query(
                "SELECT climb_id FROM climb_super_formation_closures WHERE super_formation_id = $1",
                &[&self.0],
            )
            .await?;
        let climbs = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climb))
            .collect::<Result<Vec<Climb>, _>>()?;

        Ok(climbs)
    }
}
