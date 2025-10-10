use async_graphql::{Context, Object, Result, Union, ID};

use crate::{schema::grade::Grade, AppData, WithAppData};

use super::{ascent::Ascent, crag::Crag, formation::Formation, region::Region, sector::Sector};

#[derive(Union)]
enum ClimbParent {
    Region(Region),
    Crag(Crag),
    Sector(Sector),
    Formation(Formation),
}

pub struct Climb(pub i32);

#[Object]
impl Climb {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn ascents(&self, ctx: &Context<'_>) -> Result<Vec<Ascent>> {
        let client = ctx.db_client().await?;
        let rows = client.query(
            "
            SELECT a.id
            FROM climb.ascents as a
            INNER JOIN climb.climbs as c ON c.id = a.climb_id
            WHERE c.id = $1
            ",
            &[&self.0]
            )
            .await?;

        let ascents = rows
            .iter()
            .filter_map(|r| r.try_get::<_, i32>(0).ok())
            .map(Ascent)
            .collect::<Vec<_>>();

        Ok(ascents)
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
            .query_one("SELECT name FROM climb.climbs WHERE id = $1", &[&self.0])
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
            .query_one("SELECT description FROM climb.climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|description| description.to_string()))
    }

    async fn grades(&self, ctx: &Context<'_>) -> Result<Vec<Grade>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => return Err("Database connection is not available".into()),
        };

        let row = client
            // Cast to TEXT[], if at some point there are Rust bindings for pg_climb, then this can
            // be improved
            .query_one(
                "
                SELECT grades::TEXT[]
                FROM climb.climbs
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let grade_strs: Vec<String> = row.get(0);

        let grades: Vec<Grade> = grade_strs
            .into_iter()
            .filter_map(|s| s.parse::<Grade>().ok())
            .collect();

        Ok(grades)
    }

    async fn parent(&self, ctx: &Context<'_>) -> Result<Option<ClimbParent>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => return Err("Database connection is not available".into()),
        };

        let row = client
            .query_one(
                "
                SELECT region_id, crag_id, sector_id, formation_id
                FROM climb.climbs
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let region_id: Option<i32> = row.try_get(0)?;
        let crag_id: Option<i32> = row.try_get(1)?;
        let sector_id: Option<i32> = row.try_get(2)?;
        let formation_id: Option<i32> = row.try_get(3)?;

        let parent = match (region_id, crag_id, sector_id, formation_id) {
            (Some(id), None, None, None) => Some(ClimbParent::Region(Region(id))),
            (None, Some(id), None, None) => Some(ClimbParent::Crag(Crag(id))),
            (None, None, Some(id), None) => Some(ClimbParent::Sector(Sector(id))),
            (None, None, None, Some(id)) => Some(ClimbParent::Formation(Formation(id))),
            (None, None, None, None) => None,
            _ => return Err("Formation has multiple parents, which violates schema".into()),
        };

        Ok(parent)
    }
}
