use async_graphql::{Context, Error, Object, Result, Union, ID};
use deadpool_postgres::Transaction;

use crate::{schema::types::{geometry::Point2D, spline::{BasisSpline, BasisSplineInput}, topo::features::{FeatureId, PathFeature, PathGeometry, TopoPathGeometryInput}}, AppData};

pub struct PathFeatureMutationRoot {
    // TODO not exposed to GraphQL, perhaps ID isn't the right type here?
    pub id: ID,
}

#[derive(Union)]
#[graphql(name = "TopoFeatureMutationRoot")]
pub enum MutationRoot {
    Path(PathFeatureMutationRoot),
}

#[Object]
impl PathFeatureMutationRoot {
    async fn assign_climb(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<PathFeature> {
        let appdata = ctx.data::<AppData>()?;
        let mut client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => return Err("Database connection is not available".into()),
        };

        let climb_id: i32 = id.parse().map_err(|_| "Invalid climb ID")?;

        // TODO This should really be in the constructor of (or the match when creating) the path feature mutation root
        let feature_id = FeatureId::try_from(self.id.clone())?;
        let feature_id = match feature_id {
            FeatureId::Path(id) => id,
            FeatureId::Image(_) => return Err("Invalid ID".into()),
        };

        let tx = client.transaction().await?;

        tx.execute(
            "
            UPDATE topo_path_features
            SET climb_id = $1
            WHERE id = $2;
            ",
            &[&climb_id, &feature_id],
        )
            .await?;

        let row = tx
            .query_one(
                "
                SELECT id, climb_id, (geometry).*
                FROM topo_path_features
                WHERE id = $1;
                ",
                &[&feature_id],
            )
            .await?;

        tx.commit().await?;

        let id: i32 = row.get("id");
        let climb_id: i32 = row.get("climb_id");
        let degree: i32 = row.get("degree");
        let degree: u32 = u32::try_from(degree)
            .map_err(|_| Error::new("degree must be non-negative"))?;
        let knots: Vec<f64> = row.get("knots");
        let control_points: Vec<Point2D> = row
            .get::<_, Vec<geo_types::Point<f64>>>("control_points")
            .into_iter()
            .map(Point2D::from)
            .collect();
        let basis_spline = BasisSpline { degree, knots, control_points };
        let geometry: PathGeometry = PathGeometry::BasisSpline(basis_spline);

        Ok(PathFeature {
            id: FeatureId::Path(id),
            climb_id,
            geometry,
        })
    }

    async fn update_geometry(
        &self,
        ctx: &Context<'_>,
        geometry: TopoPathGeometryInput,
    ) -> Result<PathFeature> {
        let appdata = ctx.data::<AppData>()?;
        let mut client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => return Err("Database connection is not available".into()),
        };

        let feature_id = FeatureId::try_from(self.id.clone())?;
        let feature_id = match feature_id {
            FeatureId::Path(id) => id,
            FeatureId::Image(_) => return Err("Invalid ID".into()),
        };

        let tx = client.transaction().await?;

        update_path_feature(&tx, feature_id, geometry).await?;

        let row = tx
            .query_one(
                "
                SELECT id, climb_id, (geometry).*
                FROM topo_path_features
                WHERE id = $1;
                ",
                &[&feature_id],
            )
            .await?;

        tx.commit().await?;

        let id: i32 = row.get("id");
        let climb_id: i32 = row.get("climb_id");
        let degree: i32 = row.get("degree");
        let degree: u32 = u32::try_from(degree)
            .map_err(|_| Error::new("degree must be non-negative"))?;
        let knots: Vec<f64> = row.get("knots");
        let control_points: Vec<Point2D> = row
            .get::<_, Vec<geo_types::Point<f64>>>("control_points")
            .into_iter()
            .map(Point2D::from)
            .collect();
        let basis_spline = BasisSpline { degree, knots, control_points };
        let geometry: PathGeometry = PathGeometry::BasisSpline(basis_spline);

        Ok(PathFeature {
            id: FeatureId::Path(id),
            climb_id,
            geometry,
        })
    }
}

async fn update_path_feature(t: &Transaction<'_>, id: i32, geometry: TopoPathGeometryInput) -> Result<()> {
    match geometry {
        TopoPathGeometryInput::BasisSpline(BasisSplineInput {
            degree,
            knots,
            control_points,
        }) => {
            let degree = i32::try_from(degree)?;
            let pg_points: Vec<_> = control_points
                .into_iter()
                .map(geo_types::Point::<f64>::from)
                .collect();

            t.execute(
                "
                UPDATE topo_path_features
                SET geometry = ROW($2, $3, $4)::basis_spline
                WHERE id = $1;
                ",
                &[&id, &degree, &knots, &pg_points],
            ).await?;
        }
    }

    Ok(())
}

