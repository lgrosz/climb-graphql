use async_graphql::{Context, Object, Result, ID};

use crate::{schema::Image, AppData};

pub struct ImageMutationRoot {
    pub id: ID,
}

#[Object]
impl ImageMutationRoot {
    async fn tag_formation(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Formation ID")] id: ID,
    ) -> Result<Image> {
        let appdata = ctx.data::<AppData>()?;
        let client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let image_id: i32 = self.id.0.parse().map_err(|_| "Invalid image ID format")?;
        let formation_id: i32 = id.0.parse().map_err(|_| "Invalid formation ID format")?;

        client
            .execute(
                "
                INSERT INTO climb.formations_in_image (image_id, formation_id)
                VALUES ($1, $2)
                ",
                &[&image_id, &formation_id],
            )
            .await?;

        Ok(Image(image_id))
    }

    async fn untag_formation(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Formation ID")] id: ID,
    ) -> Result<Image> {
        let appdata = ctx.data::<AppData>()?;
        let client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let image_id: i32 = self.id.0.parse().map_err(|_| "Invalid image ID format")?;
        let formation_id: i32 = id.0.parse().map_err(|_| "Invalid formation ID format")?;

        client
            .execute(
                "
                DELETE FROM climb.formations_in_image
                WHERE image_id = $1 AND formation_id = $2
                ",
                &[&image_id, &formation_id],
            )
            .await?;

        Ok(Image(image_id))
    }
}
