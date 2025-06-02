use async_graphql::{Context, Object, Result, ID};

use crate::AppData;

pub struct Topo(pub i32);

#[Object]
impl Topo {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn title(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT title
                FROM topos
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|s| s.to_string()))
    }

    async fn width(&self, ctx: &Context<'_>) -> Result<f64> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT width
                FROM topos
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let width: f64 = result.get(0); 

        Ok(width)
    }

    async fn height(&self, ctx: &Context<'_>) -> Result<f64> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT height
                FROM topos
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let height: f64 = result.get(0); 

        Ok(height)
    }
}
