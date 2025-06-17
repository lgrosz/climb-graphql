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

#[derive(SimpleObject)]
pub struct Rect {
    pub min: Point2D,
    pub max: Point2D,
}

impl From<geo_types::Rect<f64>> for Rect {
    fn from(r: geo_types::Rect<f64>) -> Self {
        Rect {
            min: Point2D {
                x: r.min().x,
                y: r.min().y,
            },
            max: Point2D {
                x: r.max().x,
                y: r.max().y,
            },
        }
    }
}

