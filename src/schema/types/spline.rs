use async_graphql::SimpleObject;

use super::geometry::Point2D;

#[derive(SimpleObject)]
pub struct BasisSpline {
    pub degree: u32,
    pub knots: Vec<f64>,
    pub control_points: Vec<Point2D>,
}

