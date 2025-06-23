use async_graphql::{Context, Object, Result, ID};
use deadpool_postgres::Transaction;

use crate::{schema::types::topo::{features::{TopoFeatureInput, TopoImageFeatureInput}, Topo}, AppData};

pub struct TopoMutationRoot {
    pub id: ID,
}

#[Object]
impl TopoMutationRoot {
    async fn add_feature(
        &self,
        ctx: &Context<'_>,
        feature: TopoFeatureInput,
    ) -> Result<Topo> {
        let appdata = ctx.data::<AppData>()?;
        let mut client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let transaction = client.transaction().await?;
        let topo_id: i32 = self.id.0.parse()?;

        match feature {
            TopoFeatureInput::Image(input) => insert_image_feature(&transaction, input, topo_id).await?,
        }

        transaction.commit().await?;

        Ok(Topo(topo_id))
    }
}

async fn insert_image_feature(t: &Transaction<'_>, input: TopoImageFeatureInput, topo_id: i32) -> Result<()> {
    let source_crop: Option<geo_types::Rect> = input.source.map(Into::into);
    let dest_crop: geo_types::Rect = input.dest.into();

    t.execute(
        "
        INSERT INTO topo_image_features (source_crop, dest_crop, image_id, topo_id)
        VALUES ($1, $2, $3, $4)
        ",
        &[&source_crop, &dest_crop, &input.image_id, &topo_id]).await?;

    Ok(())
}

