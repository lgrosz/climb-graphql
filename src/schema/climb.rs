use std::str::FromStr;

use async_graphql::{Context, Object, Result, SimpleObject, Union, ID};

use crate::schema::area::Area;
use crate::schema::formation::Formation;
use crate::schema::fontainebleau_grade::FontainebleauGrade;
use crate::schema::vermin_grade::VerminGrade;
use crate::schema::yosemite_decimal_grade::YosemiteDecimalGrade;
use crate::AppData;

#[derive(Union)]
enum ClimbParent {
    Area(Area),
    Formation(Formation),
}

#[derive(SimpleObject)]
struct ClimbFontainebleauGrade {
    pub value: FontainebleauGrade,
}

#[derive(SimpleObject)]
struct ClimbYosemiteDecimalGrade {
    pub value: YosemiteDecimalGrade,
}

#[derive(SimpleObject)]
struct ClimbVerminGrade {
    pub value: VerminGrade,
}

#[derive(Union)]
enum ClimbGrade {
    Vermin(ClimbVerminGrade),
    Fontainebleau(ClimbFontainebleauGrade),
    YosemiteDecimal(ClimbYosemiteDecimalGrade),
}

#[derive(Debug, PartialEq, Eq)]
struct ParseClimbGradeError;

impl FromStr for ClimbGrade {
    type Err = ParseClimbGradeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        VerminGrade::from_str(s)
            .map(|v| {
                ClimbGrade::Vermin(ClimbVerminGrade { value: v })
            })
        .or_else(|_| {
            FontainebleauGrade::from_str(s)
                .map(|f| {
                    ClimbGrade::Fontainebleau(ClimbFontainebleauGrade { value: f })
                })
        })
        .or_else(|_| {
            YosemiteDecimalGrade::from_str(s)
                .map(|y| {
                    ClimbGrade::YosemiteDecimal(ClimbYosemiteDecimalGrade { value: y })
                })
        })
        .map_err(|_| {
            ParseClimbGradeError
        })
    }
}

pub struct Climb(pub i32);

#[Object]
impl Climb {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT name FROM climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn description<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one("SELECT description FROM climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|description| description.to_string()))
    }

    async fn grades<'a>(&self, ctx: &Context<'a>) -> Result<Vec<ClimbGrade>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let value: Vec<ClimbGrade> = client
            // TODO since grade doesn't implement binary functions, we must request the text
            // format, when grade _does_ implement these, the ::TEXT is not needed
            .query(
                "
                SELECT grade::TEXT
                FROM climb_grades
                WHERE climb_id = $1
                ",
                &[&self.0],
            )
            .await?
            .into_iter()
            .filter_map(|row| {
                let grade_str: String = row.get(0);
                match grade_str.parse::<ClimbGrade>() {
                    Ok(grade) => Some(grade),
                    Err(_) => None,
                }
            })
            .collect();

        Ok(value)
    }

    async fn parent<'a>(&self, ctx: &Context<'a>) -> Result<Option<ClimbParent>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_opt(
                // TODO The JOIN is unecessary since we know only one can exist given the checks.
                // Is there a more effecient method?
                "
                SELECT sac.super_area_id, sfc.super_formation_id
                FROM climb_super_area_closures sac
                FULL JOIN climb_super_formation_closures sfc 
                ON sac.climb_id = sfc.climb_id
                WHERE sac.climb_id = $1 OR sfc.climb_id = $1
                ",
                &[&self.0],
            )
            .await?;

        if let Some(row) = result {
            match (
                row.try_get::<_, Option<i32>>(0)?,
                row.try_get::<_, Option<i32>>(1)?,
            ) {
                (Some(super_area_id), _) => {
                    return Ok(Some(ClimbParent::Area(Area(super_area_id))))
                }
                (_, Some(super_formation_id)) => {
                    return Ok(Some(ClimbParent::Formation(Formation(super_formation_id))))
                }
                _ => {}
            }
        }

        Ok(None)
    }
}
