use async_graphql::{InputObject, SimpleObject};

use super::geometry::Point2D;

#[derive(SimpleObject)]
pub struct BasisSpline {
    pub degree: u32,
    pub knots: Vec<f64>,
    pub control_points: Vec<Point2D>,
}

#[derive(InputObject)]
pub struct BasisSplineInput {
    pub degree: u32,
    pub knots: Vec<f64>,
    pub control_points: Vec<Point2D>,
}
