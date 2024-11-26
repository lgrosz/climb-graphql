use async_graphql::{Context, Object, Result};
use deadpool_postgres::Pool;

use crate::schema::climb::Climb;
use crate::schema::formation::Formation;

pub struct Area(pub i32);

#[Object]
impl Area {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query_one("SELECT name FROM areas WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn area<'a>(&self, ctx: &Context<'a>) -> Result<Option<Area>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT super_area_id FROM area_closures WHERE area_id = $1",
                &[&self.0],
            )
            .await?;
        let value: Option<i32> = if let Some(row) = result.first() {
            row.try_get(0)?
        } else {
            None
        };

        Ok(value.map(Area))
    }

    async fn areas<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Area>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT area_id FROM area_closures WHERE super_area_id = $1",
                &[&self.0],
            )
            .await?;
        let areas = result
            .into_iter()
            .map(|row| row.try_get(0).map(Area))
            .collect::<Result<Vec<Area>, _>>()?;

        Ok(areas)
    }

    async fn formations<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Formation>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT formation_id FROM formation_super_area_closures WHERE super_area_id = $1",
                &[&self.0],
            )
            .await?;
        let formations = result
            .into_iter()
            .map(|row| row.try_get(0).map(Formation))
            .collect::<Result<Vec<Formation>, _>>()?;

        Ok(formations)
    }

    async fn climbs<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Climb>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT climb_id FROM climb_super_area_closures WHERE super_area_id = $1",
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
