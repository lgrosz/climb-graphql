use async_graphql::{Context, Object, Result, SimpleObject, Union, ID};

use crate::schema::area::Area;
use crate::schema::formation::Formation;
use crate::schema::fontainebleau_grade::FontainebleauGrade;
use crate::schema::vermin_grade::VerminGrade;
use crate::schema::yosemite_decimal_grade::YosemiteDecimalGrade;
use crate::AppData;

use super::fontainebleau_grade::FontainebleauLetter;
use super::yosemite_decimal_grade::YosemiteDecimalLetter;

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

        let verm_grades: Vec<ClimbGrade> = client
            .query(
                "
                SELECT value
                FROM climb_verm_grades
                WHERE climb_id = $1
                ",
                &[&self.0],
            )
            .await?
            .into_iter()
            .filter_map(|row| {
                row.try_get::<_, i32>(0).ok().and_then(|v| {
                    u8::try_from(v)
                        .ok()
                        .map(|value| ClimbGrade::Vermin(ClimbVerminGrade { value: VerminGrade(value) } ))
                })
            })
            .collect();

        let font_grades: Vec<ClimbGrade> = client
            .query(
                "
                SELECT value, letter, plus
                FROM climb_font_grades
                WHERE climb_id = $1
                ",
                &[&self.0],
            )
            .await?
            .into_iter()
            .filter_map(|row| {
                let number: u8 = row
                    .try_get::<_, i32>(0)
                    .ok()
                    .and_then(|v| u8::try_from(v).ok())?;

                let letter = row.try_get::<_, Option<FontainebleauLetter>>(1).ok()?;

                let plus: bool = row.try_get(2).ok()?;

                Some(ClimbGrade::Fontainebleau(ClimbFontainebleauGrade {
                    value: FontainebleauGrade {
                        number,
                        letter,
                        plus,
                    },
                }))
            })
            .collect();

        let yds_grades: Vec<ClimbGrade> = client
            .query(
                "
                SELECT value, letter
                FROM climb_yds_grades
                WHERE climb_id = $1
                ",
                &[&self.0],
            )
            .await?
            .into_iter()
            .filter_map(|row| {
                let grade: u8 = row
                    .try_get::<_, i32>(0)
                    .ok()
                    .and_then(|v| u8::try_from(v).ok())?;

                let letter: Option<YosemiteDecimalLetter> =
                    row.try_get::<_, Option<YosemiteDecimalLetter>>(1).ok()?;

                Some(ClimbGrade::YosemiteDecimal(ClimbYosemiteDecimalGrade {
                    value: YosemiteDecimalGrade {
                        grade,
                        letter
                    },
                }))
            })
            .collect();

        let mut all_grades = verm_grades;
        all_grades.extend(font_grades);
        all_grades.extend(yds_grades);

        Ok(all_grades)
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
