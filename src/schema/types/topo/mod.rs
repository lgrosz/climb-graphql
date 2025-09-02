use async_graphql::{Context, Error, Object, Result, ID};

use crate::AppData;
use features::{FeatureId, ImageFeature, PathFeature, PathGeometry, TopoFeature};

use super::{geometry::{Point2D, Rect}, spline::BasisSpline};

pub mod features;

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
                FROM topo.topos
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
                FROM topo.topos
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
                FROM topo.topos
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let height: f64 = result.get(0); 

        Ok(height)
    }

    async fn features(&self, ctx: &Context<'_>) -> Result<Vec<TopoFeature>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let mut all_features = Vec::new();

        all_features.extend(get_path_features(&client, self.0).await?);
        all_features.extend(get_image_features(&client, self.0).await?);

        Ok(all_features)
    }
}

async fn get_path_features(client: &deadpool::managed::Object<deadpool_postgres::Manager>, topo_id: i32) -> Result<Vec<TopoFeature>> {
    let result = client
        .query(
            // TODO I tried to implement FromSql for BasisSpline, but I was having trouble
            // getting the data out of the underlying composite-type
            "
            SELECT id, climb_id, (geometry).*
            FROM topo.path_features
            WHERE topo_id = $1
            ",
            &[&topo_id],
        )
        .await?;

    let features: Vec<TopoFeature> = result
        .into_iter()
        .map(|row| {
            let id = FeatureId::Path(row.get("id"));
            let climb_id: i32 = row.get("climb_id");
            let degree: i32 = row.get("degree");
            let degree: u32 = u32::try_from(degree)
                .map_err(|_| Error::new("degree must be non-negative"))?;
            let knots: Vec<f64> = row.get("knots");
            let control_points: Vec<Point2D> = row
                .get::<_, Vec<geo_types::Point<f64>>>("points")
                .into_iter()
                .map(Point2D::from)
                .collect();

            let basis_spline = BasisSpline { degree, knots, control_points };
            let geometry: PathGeometry = PathGeometry::BasisSpline(basis_spline);

            Ok(TopoFeature::Path(PathFeature { id, climb_id, geometry }))
        })
    .collect::<Result<Vec<TopoFeature>, async_graphql::Error>>()?;

    Ok(features)
}

async fn get_image_features(client: &deadpool::managed::Object<deadpool_postgres::Manager>, topo_id: i32) -> Result<Vec<TopoFeature>> {
    let result = client
        .query(
            "
            SELECT id, image_id, source_crop, dest_crop
            FROM topo.image_features
            WHERE topo_id = $1
            ",
            &[&topo_id],
        )
        .await?;

    let features: Vec<TopoFeature> = result
        .into_iter()
        .map(|row| {
            let id = FeatureId::Image(row.get("id"));
            let image_id = row.get("image_id");

            let source: Option<Rect> = row
                .get::<_, Option<geo_types::Rect>>("source_crop")
                .map(Rect::from);

            let dest: Rect = row
                .get::<_, geo_types::Rect>("dest_crop")
                .into();

            Ok(TopoFeature::Image(ImageFeature { id, image_id, source, dest }))
        })
    .collect::<Result<Vec<TopoFeature>, async_graphql::Error>>()?;

    Ok(features)
}

