use async_graphql::{Context, Object, ID, Result};

use crate::AppData;

use super::region::Region;

pub struct Crag(pub i32);

#[Object]
impl Crag {
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
            .query_one("SELECT name FROM climb.crags WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn region(&self, ctx: &Context<'_>) -> Result<Option<Region>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT region_id FROM climb.crags WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<i32> = result.try_get(0)?;

        Ok(value.map(Region))
    }
}

