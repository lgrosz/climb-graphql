use async_graphql::{Object, Union};

use crate::schema::{
    climb::Climb,
    types::spline::BasisSpline,
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

#[derive(Union)]
pub enum TopoFeature {
    Path(PathFeature),
}

