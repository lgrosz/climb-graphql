use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl From<geo_types::Point<f64>> for Point2D {
    fn from(p: geo_types::Point<f64>) -> Self {
        Point2D { x: p.x(), y: p.y() }
    }
}

