use async_graphql::{Object, Union};

use crate::schema::{
    climb::Climb,
    types::{geometry::Rect, spline::BasisSpline},
    Image,
};

#[derive(Union)]
#[graphql(name = "TopoPathGeometry")]
pub enum PathGeometry {
    BasisSpline(BasisSpline),
}

pub struct PathFeature {
    pub climb_id: i32,
    pub geometry: PathGeometry,
}

#[Object(name = "TopoPathFeature")]
impl PathFeature {
    async fn climb(&self) -> Climb {
        Climb(self.climb_id)
    }

    async fn geometry(&self) -> &PathGeometry {
        &self.geometry
    }
}

pub struct ImageFeature {
    pub image_id: i32,
    pub source: Option<Rect>,
    pub dest: Rect,
}

#[Object(name = "TopoImageFeature")]
impl ImageFeature {
    async fn image(&self) -> Image {
        Image(self.image_id)
    }

    async fn source(&self) -> &Option<Rect> {
        &self.source
    }

    async fn dest(&self) -> &Rect {
        &self.dest
    }
}

#[derive(Union)]
pub enum TopoFeature {
    Path(PathFeature),
    Image(ImageFeature),
}

