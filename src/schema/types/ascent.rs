use std::str::FromStr;

use async_graphql::{Context, Error, Object, Result, ID};

use crate::{schema::scalars::date_range::DateRange, WithAppData};

use super::{ascent_party::AscentParty, climb::Climb};

pub struct Ascent(pub i32);

#[Object]
impl Ascent {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn climb(&self, ctx: &Context<'_>) -> Result<Climb> {
        let client = ctx.db_client().await?;
        let row = client.query_one("SELECT climb_id FROM climb.ascents WHERE id = $1", &[&self.0]).await?;
        Ok(row.try_get(0).map(Climb)?)
    }

    async fn ascent_window(&self, ctx: &Context<'_>) -> Result<Option<DateRange>> {
        let client = ctx.db_client().await?;
        let row = client.query_one("SELECT ascent_window::text FROM climb.ascents WHERE id = $1", &[&self.0]).await?;
        let s: Option<String> = row.try_get(0)?;

        let range = s
            .map(|s| DateRange::from_str(&s))
            .transpose()?;

        Ok(range)
    }

    async fn party(&self, ctx: &Context<'_>) -> Result<AscentParty> {
        let client = ctx.db_client().await?;
        let rows = client
            .query(
                "
                SELECT a.members_complete, am.climber_id
                FROM climb.ascents AS a
                LEFT JOIN climb.ascent_members AS am ON a.id = am.ascent_id
                WHERE a.id = $1
                ",
                &[&self.0],
            )
            .await?;

        let complete: bool = rows
            .first()
            .ok_or_else(|| Error::new("Ascent not found"))?
            .try_get("members_complete")?;

        let member_ids = rows
            .iter()
            .filter_map(|row| row.try_get::<_, i32>("climber_id").ok())
            .collect::<Vec<_>>();

        Ok(AscentParty { complete, member_ids })
    }

    async fn first_ascent(&self, ctx: &Context<'_>) -> Result<bool> {
        let client = ctx.db_client().await?;
        let row = client.query_one("SELECT first_ascent FROM climb.ascents WHERE id = $1", &[&self.0]).await?;
        Ok(row.try_get::<_, bool>(0)?)
    }

    async fn verified(&self, ctx: &Context<'_>) -> Result<bool> {
        let client = ctx.db_client().await?;
        let row = client.query_one("SELECT verified FROM climb.ascents WHERE id = $1", &[&self.0]).await?;
        Ok(row.try_get::<_, bool>(0)?)
    }
}

