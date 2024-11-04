use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use deadpool_postgres::Pool;

use crate::schema::area::Area;
use crate::schema::climb::Climb;

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "CoordinateInput")]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

pub struct Formation(pub i32);

#[Object]
impl Formation {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query_one("SELECT name FROM formations WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn location<'a>(&self, _ctx: &Context<'a>) -> Result<Option<Coordinate>> {
        todo!()
    }

    async fn area<'a>(&self, ctx: &Context<'a>) -> Result<Option<Area>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT super_area_id FROM formation_super_area_closures WHERE formation_id = $1",
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

    async fn formation<'a>(&self, ctx: &Context<'a>) -> Result<Option<Formation>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client.query("SELECT super_formation_id FROM formation_super_formation_closures WHERE formation_id = $1", &[&self.0]).await?;
        let value: Option<i32> = if let Some(row) = result.first() {
            row.try_get(0)?
        } else {
            None
        };

        Ok(value.map(Formation))
    }

    async fn formations<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Formation>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client.query("SELECT formation_id FROM formation_super_formation_closures WHERE super_formation_id = $1", &[&self.0]).await?;
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
