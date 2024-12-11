use async_graphql::{Context, Enum, InputObject, Object, OneofObject, Result, SimpleObject, Union};

use area::Area;
use climb::Climb;
use formation::{Coordinate, Formation};
use postgres_types::ToSql;

use crate::AppData;

pub mod area;
pub mod climb;
pub mod formation;

pub struct QueryRoot;

#[derive(InputObject)]
struct VerminGradeInput {
    pub value: u8,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug, ToSql)]
#[postgres(name = "font_letter")]
enum FontainebleauLetterInput {
    #[postgres(name = "A")]
    A,
    #[postgres(name = "B")]
    B,
    #[postgres(name = "C")]
    C,
}

#[derive(InputObject)]
struct FontainebleauGradeInput {
    pub value: u8,
    pub letter: Option<FontainebleauLetterInput>,
    pub plus: bool,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug, ToSql)]
#[postgres(name = "yds_letter")]
enum YosemiteDecimalLetterInput {
    #[postgres(name = "a")]
    A,
    #[postgres(name = "b")]
    B,
    #[postgres(name = "c")]
    C,
    #[postgres(name = "d")]
    D,
}

#[derive(InputObject)]
struct YosemiteDecimalGradeInput {
    pub value: u8,
    pub letter: Option<YosemiteDecimalLetterInput>,
}

#[derive(OneofObject)]
enum GradeInput {
    Vermin(VerminGradeInput),
    Fontainebleau(FontainebleauGradeInput),
    YosemiteDecimal(YosemiteDecimalGradeInput),
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
enum GradeOperation {
    Add,
    Remove,
}

struct Image(pub i32);

#[derive(Union)]
enum ImageSource {
    S3(S3ImageSource),
}

#[derive(SimpleObject)]
struct S3ImageSource {
    pub bucket: String,
    pub object: String,
}

impl S3ImageSource {
    fn from_row(row: &tokio_postgres::Row) -> Self {
        Self {
            bucket: row.get("bucket"),
            object: row.get("object"),
        }
    }
}

#[Object]
impl Image {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn sources<'a>(&self, ctx: &Context<'a>) -> Result<Vec<ImageSource>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = client
            .query(
                "
                SELECT a.bucket, a.object
                FROM images
                LEFT JOIN s3_image_sources AS a ON images.id = a.image_id
                WHERE images.id = $1
                ",
                &[&self.0],
            )
            .await?;

        let sources = result.into_iter()
            .map(|row| ImageSource::S3(S3ImageSource::from_row(&row)))
            .collect();

        Ok(sources)
    }
}

#[Object]
impl QueryRoot {
    async fn areas<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] area_id: Option<i32>,
    ) -> Result<Vec<Area>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = if let Some(area_id) = area_id {
            // If `area_id` is provided, find areas with this specific parent
            client
                .query(
                    "
                    SELECT a.id
                    FROM areas AS a
                    INNER JOIN area_closures AS sa ON a.id = sa.area_id
                    WHERE sa.super_area_id = $1
                    ",
                    &[&area_id],
                )
                .await?
        } else {
            // If `area_id` is None, find areas with no parent (top-level areas)
            client
                .query(
                    "
                    SELECT a.id
                    FROM areas AS a
                    LEFT JOIN area_closures AS sa ON a.id = sa.area_id
                    WHERE sa.super_area_id IS NULL
                    ",
                    &[],
                )
                .await?
        };

        let areas = result
            .into_iter()
            .map(|row| row.try_get(0).map(Area))
            .collect::<Result<Vec<Area>, _>>()?;

        Ok(areas)
    }

    async fn area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: i32,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        // Just check for existence
        client
            .query_one("SELECT 1 FROM areas WHERE id = $1", &[&id])
            .await?;

        Ok(Area(id))
    }

    async fn climbs<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] area_id: Option<i32>,
        #[graphql(desc = "Parent formation id")] formation_id: Option<i32>,
    ) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = if let Some(area_id) = area_id {
            // If `area_id` is provided, find climbs with this specific parent
            client
                .query(
                    "
                    SELECT c.id
                    FROM climbs AS c
                    INNER JOIN climb_super_area_closures AS sa ON c.id = sa.climb_id
                    WHERE sa.super_area_id = $1
                    ",
                    &[&area_id],
                )
                .await?
        } else if let Some(formation_id) = formation_id {
            // If `formation_id` is provided, find climbs with this specific parent
            client
                .query(
                    "
                    SELECT c.id
                    FROM climbs AS c
                    INNER JOIN climb_super_formation_closures AS sf ON c.id = sf.climb_id
                    WHERE sf.super_formation_id = $1
                    ",
                    &[&formation_id],
                )
                .await?
        } else {
            // If `area_id` and `formation_id` are None, find climbs with no parent (top-level climbs)
            client
                .query(
                    "
                    SELECT c.id
                    FROM climbs AS c
                    LEFT JOIN climb_super_area_closures AS sa ON c.id = sa.climb_id
                    LEFT JOIN climb_super_formation_closures AS sf ON c.id = sf.climb_id
                    WHERE sa.super_area_id IS NULL AND sf.super_formation_id IS NULL
                    ",
                    &[],
                )
                .await?
        };

        let climbs = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climb))
            .collect::<Result<Vec<Climb>, _>>()?;

        Ok(climbs)
    }

    async fn climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        // Just check for existence
        client
            .query_one("SELECT 1 FROM climbs WHERE id = $1", &[&id])
            .await?;

        Ok(Climb(id))
    }

    async fn formations<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] area_id: Option<i32>,
        #[graphql(desc = "Parent formation id")] formation_id: Option<i32>,
    ) -> Result<Vec<Formation>> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let result = if let Some(area_id) = area_id {
            // If `area_id` is provided, find formations with this specific parent
            client
                .query(
                    "
                    SELECT f.id
                    FROM formations AS f
                    INNER JOIN formation_super_area_closures AS sa ON f.id = sa.formation_id
                    WHERE sa.super_area_id = $1
                    ",
                    &[&area_id],
                )
                .await?
        } else if let Some(formation_id) = formation_id {
            // If `formation_id` is provided, find formations with this specific parent
            client
                .query(
                    "
                    SELECT f.id
                    FROM formations AS f
                    INNER JOIN formation_super_formation_closures AS sf ON f.id = sf.formation_id
                    WHERE sf.super_formation_id = $1
                    ",
                    &[&formation_id],
                )
                .await?
        } else {
            // If `area_id` and `formation_id` are None, find formations with no parent (top-level formations)
            client
                .query(
                    "
                    SELECT f.id
                    FROM formations AS f
                    LEFT JOIN formation_super_area_closures AS sa ON f.id = sa.formation_id
                    LEFT JOIN formation_super_formation_closures AS sf ON f.id = sf.formation_id
                    WHERE sa.super_area_id IS NULL AND sf.super_formation_id IS NULL
                    ",
                    &[],
                )
                .await?
        };

        let formations = result
            .into_iter()
            .map(|row| row.try_get(0).map(Formation))
            .collect::<Result<Vec<Formation>, _>>()?;

        Ok(formations)
    }

    async fn formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: i32,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        // Just check for existence
        client
            .query_one("SELECT 1 FROM formations WHERE id = $1", &[&id])
            .await?;

        Ok(Formation(id))
    }

    async fn image<'a>(
        &self,
        ctx: &Context<'a>,
        id: i32,
    ) -> Result<Image> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        // Just check for existence
        client
            .query_one("SELECT 1 FROM images WHERE id = $1", &[&id])
            .await?;

        Ok(Image(id))
    }

    async fn image_source_url<'a>(
        &self,
        ctx: &Context<'a>,
        id: i32,
        #[graphql(validator(min_length = 1))] name: String,
    ) -> Result<String> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        // Ensure id is valid
        client
            .query_one("SELECT * FROM images WHERE id = $1", &[&id])
            .await?;

        // Error out if there already exists a source for this image
        if (client
            .query_opt("SELECT 1 FROM s3_image_sources WHERE image_id = $1", &[&id])
            .await?)
            .is_some()
        {
            return Err(async_graphql::Error::new("Source already exists"));
        }

        Ok(format!("{}/{}", id, name))
    }

    async fn s3_get<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(validator(min_length = 1))] bucket: String,
        #[graphql(validator(min_length = 1))] object: String,
    ) -> Result<String> {
        let data = ctx.data::<AppData>()?;
        let s3 = data.s3_pools.get(&bucket.to_string())
            .ok_or(format!("No {} bucket configured", bucket))?
            .get().await?;

        Ok(s3.presign_get(object.to_string(), 300, None).await?)
    }
}

#[derive(OneofObject)]
enum ClimbParentInput {
    Area(i32),
    Formation(i32),
}

#[derive(OneofObject)]
enum FormationParentInput {
    Area(i32),
    Formation(i32),
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn add_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area name")] name: Option<String>,
        #[graphql(desc = "Super area id")] super_area_id: Option<i32>,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let mut client = data.pg_pool.get().await?;

        let transaction = client.transaction().await?;

        let area_id = transaction
            .query_one(
                "INSERT INTO areas (name) VALUES ($1) RETURNING id",
                &[&name],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(super_area_id) = super_area_id {
            transaction
                .execute(
                    "INSERT INTO area_closures (area_id, super_area_id) VALUES ($1, $2)",
                    &[&area_id, &super_area_id],
                )
                .await?;
        }

        transaction.commit().await?;

        Ok(Area(area_id))
    }

    async fn rename_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: i32,
        #[graphql(desc = "Area name")] name: Option<String>,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let area_id = client
            .query_one(
                "UPDATE areas SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Area(area_id))
    }

    async fn move_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: i32,
        #[graphql(desc = "Super area id")] super_area_id: Option<i32>,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        if let Some(super_area_id) = super_area_id {
            client
                .execute(
                    "
                    INSERT INTO area_closures (area_id, super_area_id)
                    VALUES ($1, $2)
                    ON CONFLICT (area_id)
                    DO UPDATE SET
                    super_area_id = EXCLUDED.super_area_id
                    ",
                    &[&id, &super_area_id],
                )
                .await?;
        } else {
            client
                .execute("DELETE FROM area_closures WHERE area_id = $1", &[&id])
                .await?;
        }

        Ok(Area(id))
    }

    async fn remove_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: i32,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        client
            .execute("DELETE FROM areas WHERE id = $1", &[&id])
            .await?;

        Ok(Area(id))
    }

    async fn add_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb name")] name: Option<String>,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParentInput>,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let mut client = data.pg_pool.get().await?;

        let transaction = client.transaction().await?;

        let id = transaction
            .query_one(
                "INSERT INTO climbs (name) VALUES ($1) RETURNING id",
                &[&name],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(parent) = parent {
            match parent {
                ClimbParentInput::Area(area_id) => {
                    transaction
                        .execute(
                            "INSERT INTO climb_super_area_closures (climb_id, super_area_id) VALUES ($1, $2)",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                ClimbParentInput::Formation(formation_id) => {
                    transaction
                        .execute(
                            "INSERT INTO climb_super_formation_closures (climb_id, super_formation_id) VALUES ($1, $2)",
                            &[&id, &formation_id],
                        )
                        .await?;
                }
            }
        }

        transaction.commit().await?;

        Ok(Climb(id))
    }

    async fn rename_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
        #[graphql(desc = "Climb name")] name: Option<String>,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let id = client
            .query_one(
                "UPDATE climbs SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climb(id))
    }

    async fn move_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParentInput>,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let mut client = data.pg_pool.get().await?;

        let transaction = client.transaction().await?;

        if let Some(parent) = parent {
            match parent {
                ClimbParentInput::Area(area_id) => {
                    transaction
                        .execute(
                            "DELETE FROM climb_super_formation_closures WHERE climb_id = $1",
                            &[&id],
                        )
                        .await?;

                    transaction
                        .execute(
                            "
                            INSERT INTO climb_super_area_closures (climb_id, super_area_id)
                            VALUES ($1, $2)
                            ON CONFLICT (climb_id)
                            DO UPDATE SET
                            super_area_id = EXCLUDED.super_area_id
                            ",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                ClimbParentInput::Formation(formation_id) => {
                    transaction
                        .execute(
                            "DELETE FROM climb_super_area_closures WHERE climb_id = $1",
                            &[&id],
                        )
                        .await?;

                    transaction
                        .execute(
                            "
                            INSERT INTO climb_super_formation_closures (climb_id, super_formation_id)
                            VALUES ($1, $2)
                            ON CONFLICT (climb_id)
                            DO UPDATE SET
                            super_formation_id = EXCLUDED.super_formation_id
                            ",
                            &[&id, &formation_id],
                        )
                        .await?;
                }
            }
        }

        transaction.commit().await?;

        Ok(Climb(id))
    }

    async fn grade_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
        #[graphql(desc = "Grade")] grade: GradeInput,
        #[graphql(desc = "Operation")] operation: GradeOperation,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        match operation {
            GradeOperation::Add => match grade {
                GradeInput::Vermin(VerminGradeInput { value }) => {
                    client
                        .execute(
                            "
                            INSERT INTO climb_verm_grades (climb_id, value)
                            VALUES ($1, $2) ON CONFLICT DO NOTHING
                            ",
                            &[&id, &(value as i32)],
                        )
                        .await?;
                }
                GradeInput::Fontainebleau(FontainebleauGradeInput {
                    value,
                    letter,
                    plus,
                }) => {
                    client
                        .execute(
                            "
                            INSERT INTO climb_font_grades (climb_id, value, letter, plus)
                            VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING
                            ",
                            &[&id, &(value as i32), &letter, &plus],
                        )
                        .await?;
                }
                GradeInput::YosemiteDecimal(YosemiteDecimalGradeInput { value, letter }) => {
                    client
                        .execute(
                            "INSERT INTO climb_yds_grades (climb_id, value, letter)
                            VALUES ($1, $2, $3) ON CONFLICT DO NOTHING
                            ",
                            &[&id, &(value as i32), &letter],
                        )
                        .await?;
                }
            },
            GradeOperation::Remove => match grade {
                GradeInput::Vermin(VerminGradeInput { value }) => {
                    client
                        .execute(
                            "
                            DELETE FROM climb_verm_grades
                            WHERE climb_id = $1 AND value = $2
                            ",
                            &[&id, &(value as i32)],
                        )
                        .await?;
                }
                GradeInput::Fontainebleau(FontainebleauGradeInput {
                    value,
                    letter,
                    plus,
                }) => {
                    client
                        .execute(
                            "
                            DELETE FROM climb_font_grades
                            WHERE climb_id = $1 AND value = $2 AND (letter IS NOT DISTINCT FROM $3) AND plus = $4
                            ",
                            &[&id, &(value as i32), &letter, &plus],
                        )
                        .await?;
                }
                GradeInput::YosemiteDecimal(YosemiteDecimalGradeInput { value, letter }) => {
                    client
                        .execute(
                            "
                            DELETE FROM climb_yds_grades
                            WHERE climb_id = $1 AND value = $2 AND (letter IS NOT DISTINCT FROM $3)
                            ",
                            &[&id, &(value as i32), &letter],
                        )
                        .await?;
                }
            },
        }

        Ok(Climb(id))
    }

    async fn remove_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        client
            .execute("DELETE FROM climbs WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Climb(id))
    }

    async fn add_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation name")] name: Option<String>,
        #[graphql(desc = "Formation location")] location: Option<Coordinate>,
        #[graphql(desc = "Formation parent")] parent: Option<FormationParentInput>,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let mut client = data.pg_pool.get().await?;

        let transaction = client.transaction().await?;

        let point =
            location.map(|coord| postgis::ewkb::Point::new(coord.longitude, coord.latitude, None));

        let id = transaction
            .query_one(
                "INSERT INTO formations (name, location) VALUES ($1, $2) RETURNING id",
                &[&name, &point],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(parent) = parent {
            match parent {
                FormationParentInput::Area(area_id) => {
                    transaction
                        .execute(
                            "INSERT INTO formation_super_area_closures (formation_id, super_area_id) VALUES ($1, $2)",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                FormationParentInput::Formation(formation_id) => {
                    transaction
                        .execute(
                            "INSERT INTO formation_super_formation_closures (formation_id, super_formation_id) VALUES ($1, $2)",
                            &[&id, &formation_id],
                        )
                        .await?;
                }
            }
        }

        transaction.commit().await?;

        Ok(Formation(id))
    }

    async fn rename_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: i32,
        #[graphql(desc = "Formation name")] name: Option<String>,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let id = client
            .query_one(
                "UPDATE formations SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Formation(id))
    }

    async fn relocate_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: i32,
        #[graphql(desc = "Formation location")] location: Option<Coordinate>,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let point =
            location.map(|coord| postgis::ewkb::Point::new(coord.longitude, coord.latitude, None));

        let id = client
            .query_one(
                "UPDATE formations SET location = $1 WHERE id = $2 RETURNING id",
                &[&point, &id],
            )
            .await?
            .try_get::<_, i32>(0)?;

        Ok(Formation(id))
    }

    async fn move_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: i32,
        #[graphql(desc = "Formation parent")] parent: Option<FormationParentInput>,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let mut client = data.pg_pool.get().await?;

        let transaction = client.transaction().await?;

        if let Some(parent) = parent {
            match parent {
                FormationParentInput::Area(area_id) => {
                    transaction
                        .execute(
                            "DELETE FROM formation_super_area_closures WHERE formation_id = $1",
                            &[&id],
                        )
                        .await?;

                    transaction
                        .execute(
                            "
                            INSERT INTO formation_super_area_closures (formation_id, super_area_id)
                            VALUES ($1, $2)
                            ON CONFLICT (formation_id)
                            DO UPDATE SET
                            super_area_id = EXCLUDED.super_area_id
                            ",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                FormationParentInput::Formation(formation_id) => {
                    transaction
                        .execute(
                            "DELETE FROM formation_super_area_closures WHERE formation_id = $1",
                            &[&id],
                        )
                        .await?;

                    transaction
                        .execute(
                            "
                            INSERT INTO formation_super_formation_closures (formation_id, super_formation_id)
                            VALUES ($1, $2)
                            ON CONFLICT (formation_id)
                            DO UPDATE SET
                            super_formation_id = EXCLUDED.super_formation_id
                            ",
                            &[&id, &formation_id],
                        )
                        .await?;
                }
            }
        }
        transaction.commit().await?;

        Ok(Formation(id))
    }

    async fn remove_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: i32,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        client
            .execute("DELETE FROM formations WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Formation(id))
    }

    async fn create_image<'a>(&self, ctx: &Context<'a>) -> Result<Image> {
        let data = ctx.data::<AppData>()?;
        let client = data.pg_pool.get().await?;

        let id = client
            .query_one("INSERT INTO images DEFAULT VALUES RETURNING id", &[])
            .await?
            .get::<_, i32>(0);

        Ok(Image(id))
    }

    async fn s3_put<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(validator(min_length = 1))] bucket: String,
        #[graphql(validator(min_length = 1))] object: String,
    ) -> Result<String> {
        let data = ctx.data::<AppData>()?;
        let s3 = data.s3_pools.get(&bucket.to_string())
            .ok_or(format!("No {} bucket configured", bucket))?
            .get().await?;

        Ok(s3.presign_put(object.to_string(), 300, None, None).await?)
    }
}
