use async_graphql::{InputObject, Interface, Object, OneofObject, Union, ID};

use crate::schema::{
    climb::Climb,
    types::{geometry::Rect, spline::{BasisSpline, BasisSplineInput}},
    Image,
};

pub enum FeatureId {
    Path(i32),
    Image(i32),
}

#[derive(Union)]
#[graphql(name = "TopoPathGeometry")]
pub enum PathGeometry {
    BasisSpline(BasisSpline),
}

pub struct PathFeature {
    pub id: FeatureId,
    pub climb_id: i32,
    pub geometry: PathGeometry,
}

impl From<&FeatureId> for ID {
    fn from(value: &FeatureId) -> Self {
        match value {
            FeatureId::Path(id) => ID(format!("path/{}", id)),
            FeatureId::Image(id) => ID(format!("image/{}", id)),
        }
    }
}

#[Object(name = "TopoPathFeature")]
impl PathFeature {
    async fn id(&self) -> ID {
        (&self.id).into()
    }

    async fn climb(&self) -> Climb {
        Climb(self.climb_id)
    }

    async fn geometry(&self) -> &PathGeometry {
        &self.geometry
    }
}

pub struct ImageFeature {
    pub id: FeatureId,
    pub image_id: i32,
    pub source: Option<Rect>,
    pub dest: Rect,
}

#[Object(name = "TopoImageFeature")]
impl ImageFeature {
    async fn id(&self) -> ID {
        (&self.id).into()
    }

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

#[derive(Interface)]
#[graphql(
    field(name = "id", ty = "ID"),
)]
pub enum TopoFeature {
    Path(PathFeature),
    Image(ImageFeature),
}

#[derive(InputObject)]
pub struct TopoImageFeatureInput {
    pub image_id: i32, // TODO Since these are public, they should be `ID`s
    pub source: Option<Rect>,
    pub dest: Rect,
}

#[derive(OneofObject)]
pub enum TopoPathGeometryInput {
    BasisSpline(BasisSplineInput),
}

#[derive(InputObject)]
pub struct TopoPathFeatureInput {
    pub climb_id: ID,
    pub geometry: TopoPathGeometryInput,
}

#[derive(OneofObject)]
pub enum TopoFeatureInput {
    Image(TopoImageFeatureInput),
    Path(TopoPathFeatureInput),
}

