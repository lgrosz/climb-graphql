use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject, Union};
use deadpool_postgres::Pool;

use crate::schema::area::Area;
use crate::schema::formation::Formation;

#[derive(Union)]
enum ClimbParent {
    Area(Area),
    Formation(Formation),
}

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

    async fn grades<'a>(&self, _ctx: &Context<'a>) -> Result<Option<Vec<Grade>>> {
        todo!()
    }

    async fn parent<'a>(&self, ctx: &Context<'a>) -> Result<Option<ClimbParent>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        let result = client
            .query_opt(
                // TODO The JOIN is unecessary since we know only one can exist given the checks.
                // Is there a more effecient method?
                "
                SELECT sac.super_area_id, sfc.super_formation_id
                FROM climb_super_area_closures sac
                FULL JOIN climb_super_formation_closures sfc 
                ON sac.climb_id = sfc.climb_id
                WHERE sac.climb_id = $1 OR sfc.climb_id = $1
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
                    return Ok(Some(ClimbParent::Area(Area(super_area_id))))
                }
                (_, Some(super_formation_id)) => {
                    return Ok(Some(ClimbParent::Formation(Formation(super_formation_id))))
                }
                _ => {}
            }
        }

        Ok(None)
    }
}
