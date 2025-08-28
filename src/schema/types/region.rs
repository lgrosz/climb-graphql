use async_graphql::{Context, Object, ID, Result};

use crate::AppData;

use super::{climb::Climb, crag::Crag, formation::Formation};

pub struct Region(pub i32);

#[Object]
impl Region {
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
            .query_one("SELECT name FROM climb.regions WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn crags(&self, ctx: &Context<'_>) -> Result<Vec<Crag>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let rows = client
            .query(
                "SELECT id FROM climb.crags WHERE region_id = $1 ORDER BY name",
                &[&self.0],
            )
            .await?;

        Ok(rows.into_iter().map(|row| Crag(row.get(0))).collect())
    }

    async fn formations(&self, ctx: &Context<'_>) -> Result<Vec<Formation>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let rows = client
            .query(
                "SELECT id FROM climb.formations WHERE region_id = $1 ORDER BY name",
                &[&self.0],
            )
            .await?;

        Ok(rows.into_iter().map(|row| Formation(row.get(0))).collect())
    }

    async fn climbs(&self, ctx: &Context<'_>) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let rows = client
            .query(
                "SELECT id FROM climb.climbs WHERE region_id = $1 ORDER BY name",
                &[&self.0],
            )
            .await?;

        Ok(rows.into_iter().map(|row| Climb(row.get(0))).collect())
    }
}

