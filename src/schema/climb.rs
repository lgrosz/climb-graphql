use async_graphql::{Context, Enum, Object, Result, SimpleObject, Union, ID};
use postgres_types::FromSql;

use crate::schema::area::Area;
use crate::schema::formation::Formation;
use crate::AppData;

#[derive(Union)]
enum ClimbParent {
    Area(Area),
    Formation(Formation),
}

#[derive(SimpleObject)]
struct VerminGrade {
    pub value: u8,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, FromSql, Debug)]
#[postgres(name = "font_letter")]
enum FontainebleauLetter {
    #[postgres(name = "A")]
    A,
    #[postgres(name = "B")]
    B,
    #[postgres(name = "C")]
    C,
}

#[derive(SimpleObject)]
struct FontainebleauGrade {
    pub value: u8,
    pub letter: Option<FontainebleauLetter>,
    pub plus: bool,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, FromSql)]
#[postgres(name = "yds_letter")]
enum YosemiteDecimalLetter {
    #[postgres(name = "a")]
    A,
    #[postgres(name = "b")]
    B,
    #[postgres(name = "c")]
    C,
    #[postgres(name = "d")]
    D,
}

#[derive(SimpleObject)]
struct YosemiteDecimalGrade {
    pub value: u8,
    pub letter: Option<YosemiteDecimalLetter>,
}

#[derive(Union)]
enum Grade {
    Vermin(VerminGrade),
    Fontainebleau(FontainebleauGrade),
    YosemiteDecimal(YosemiteDecimalGrade),
}

pub struct Climb(pub i32);

#[Object]
impl Climb {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn name<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = client
            .query_one("SELECT name FROM climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|name| name.to_string()))
    }

    async fn description<'a>(&self, ctx: &Context<'a>) -> Result<Option<String>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = client
            .query_one("SELECT description FROM climbs WHERE id = $1", &[&self.0])
            .await?;
        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|description| description.to_string()))
    }

    async fn grades<'a>(&self, ctx: &Context<'a>) -> Result<Vec<Grade>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let verm_grades: Vec<Grade> = client
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
                        .map(|value| Grade::Vermin(VerminGrade { value }))
                })
            })
            .collect();

        let font_grades: Vec<Grade> = client
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
                let value: u8 = row
                    .try_get::<_, i32>(0)
                    .ok()
                    .and_then(|v| u8::try_from(v).ok())?;

                let letter = row.try_get::<_, Option<FontainebleauLetter>>(1).ok()?;

                let plus: bool = row.try_get(2).ok()?;

                Some(Grade::Fontainebleau(FontainebleauGrade {
                    value,
                    letter,
                    plus,
                }))
            })
            .collect();

        let yds_grades: Vec<Grade> = client
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
                let value: u8 = row
                    .try_get::<_, i32>(0)
                    .ok()
                    .and_then(|v| u8::try_from(v).ok())?;

                let letter: Option<YosemiteDecimalLetter> =
                    row.try_get::<_, Option<YosemiteDecimalLetter>>(1).ok()?;

                Some(Grade::YosemiteDecimal(YosemiteDecimalGrade {
                    value,
                    letter,
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
        let client = data.pg_pool.get().await?;

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
