use async_graphql::{Context, Object, Result, ID};
use deadpool_postgres::Transaction;
use feature::PathFeatureMutationRoot;

use crate::{schema::types::{spline::BasisSplineInput, topo::{features::{FeatureId, TopoFeatureInput, TopoImageFeatureInput, TopoPathFeatureInput, TopoPathGeometryInput}, Topo}}, AppData};

pub mod feature;

pub struct TopoMutationRoot {
    pub id: ID,
}

#[Object]
impl TopoMutationRoot {
    async fn title(
        &self,
        ctx: &Context<'_>,
        title: Option<String>,
    ) -> Result<Topo> {
        let appdata = ctx.data::<AppData>()?;
        let client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let topo_id: i32 = self.id.0.parse()?;

        client.execute(
            "
            UPDATE topo.topos
            SET title = $1
            WHERE id = $2;
            ",
            &[&title, &topo_id]).await?;

        Ok(Topo(topo_id))
    }

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
            TopoFeatureInput::Path(input) => insert_path_feature(&transaction, input, topo_id).await?,
        }

        transaction.commit().await?;

        Ok(Topo(topo_id))
    }

    async fn remove_feature(
        &self,
        ctx: &Context<'_>,
        id: ID,
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
        let feature_id = FeatureId::try_from(id)?;

        match feature_id {
            FeatureId::Path(id) => remove_path_feature(&transaction, topo_id, id).await?,
            FeatureId::Image(id) => remove_image_feature(&transaction ,topo_id, id).await?,
        }

        transaction.commit().await?;

        Ok(Topo(topo_id))
    }

    async fn feature(&self, id: ID) -> feature::MutationRoot {
        // TODO check topo_path_features where topo_id matches
        feature::MutationRoot::Path(PathFeatureMutationRoot { id })
    }
}

async fn insert_image_feature(t: &Transaction<'_>, input: TopoImageFeatureInput, topo_id: i32) -> Result<()> {
    let source_crop: Option<geo_types::Rect> = input.source.map(Into::into);
    let dest_crop: geo_types::Rect = input.dest.into();

    t.execute(
        "
        INSERT INTO topo.image_features (source_crop, dest_crop, image_id, topo_id)
        VALUES ($1, $2, $3, $4)
        ",
        &[&source_crop, &dest_crop, &input.image_id, &topo_id]).await?;

    Ok(())
}

async fn insert_path_feature(t: &Transaction<'_>, input: TopoPathFeatureInput, topo_id: i32) -> Result<()> {
    match input.geometry {
        TopoPathGeometryInput::BasisSpline(BasisSplineInput {
            degree,
            knots,
            control_points,
        }) => {
            let climb_id = i32::try_from(input.climb_id)?;
            let degree = i32::try_from(degree)?;
            let pg_points: Vec<_> = control_points
                .into_iter()
                .map(geo_types::Point::<f64>::from)
                .collect();

            t.execute(
                "
                INSERT INTO topo.path_features (geometry, climb_id, topo_id)
                VALUES (ROW($1, $2, $3)::basis_spline, $4, $5)
                ",
                &[&degree, &knots, &pg_points, &climb_id, &topo_id]).await?;
        }
    }

    Ok(())
}

async fn remove_path_feature(t: &Transaction<'_>, topo_id: i32, feature_id: i32) -> Result<()> {
    t.execute(
        "
        DELETE FROM topo.path_features
        WHERE topo_id = $1 AND id = $2
        ", &[&topo_id, &feature_id]).await?;

    Ok(())
}

async fn remove_image_feature(t: &Transaction<'_>, topo_id: i32, feature_id: i32) -> Result<()> {
    t.execute(
        "
        DELETE FROM topo.image_features
        WHERE topo_id = $1 AND id = $2
        ", &[&topo_id, &feature_id]).await?;

    Ok(())
}
