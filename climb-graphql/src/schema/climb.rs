use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use deadpool_postgres::Pool;

use crate::schema::area::Area;
use crate::schema::formation::Formation;

pub struct Climb(pub i32);

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GradeType {
    Vermin,
}

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "GradeInput")]
pub struct Grade {
    #[graphql(name = "type")]
    pub grade_type: GradeType,
    pub value: String,
}

#[Object]
impl Climb {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query_one("SELECT name FROM climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn grades<'a>(&self, _ctx: &Context<'a>) -> Option<Vec<Grade>> {
        None
    }

    async fn area<'a>(&self, ctx: &Context<'a>) -> Result<Option<Area>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query(
                "SELECT super_area_id FROM climb_super_area_closures WHERE climb_id = $1",
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

        let result = client
            .query(
                "SELECT super_formation_id FROM climb_super_formation_closures WHERE climb_id = $1",
                &[&self.0],
            )
            .await?;
        let value: Option<i32> = if let Some(row) = result.first() {
            row.try_get(0)?
        } else {
            None
        };

        Ok(value.map(Formation))
    }
}
