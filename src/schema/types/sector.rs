use async_graphql::{Context, Object, ID, Result};

use crate::AppData;

use super::crag::Crag;

pub struct Sector(pub i32);

#[Object]
impl Sector {
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
            .query_one("SELECT name FROM climb.sectors WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn crag(&self, ctx: &Context<'_>) -> Result<Option<Crag>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT crag_id FROM climb.sectors WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<i32> = result.try_get(0)?;

        Ok(value.map(Crag))
    }
}
