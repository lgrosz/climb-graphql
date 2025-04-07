use std::fmt::Display;

use async_graphql::{Context, Object, OneofObject, Result, SimpleObject, ID};

use area::Area;
use climb::Climb;
use date_interval::DateInterval;
use formation::{Coordinate, Formation};
use grade::{Grade, GradeInput};
use postgres_types::ToSql;

use crate::AppData;

pub mod area;
pub mod climb;
pub mod date_interval;
pub mod formation;
pub mod grade;
pub mod fontainebleau_grade;
pub mod vermin_grade;
pub mod yosemite_decimal_grade;

pub struct QueryRoot;

impl Display for GradeInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GradeInput::Vermin(vermin_grade) => write!(f, "{}", vermin_grade),
            GradeInput::Fontainebleau(fontainebleau_grade) => write!(f, "{}", fontainebleau_grade),
            GradeInput::YosemiteDecimal(yosemite_decimal_grade) => write!(f, "{}", yosemite_decimal_grade),
        }
    }
}

impl ToSql for GradeInput {
    fn encode_format(&self, _ty: &postgres_types::Type) -> postgres_types::Format {
        postgres_types::Format::Text
    }

    fn to_sql(&self, _: &postgres_types::Type, out: &mut bytes::BytesMut) -> std::result::Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>>
    where
        Self: Sized
    {
        let encoded = self.to_string();
        out.extend_from_slice(encoded.as_bytes());
        Ok(postgres_types::IsNull::No)
    }

    fn accepts(ty: &postgres_types::Type) -> bool
    where
        Self: Sized
    {
        matches!(ty.name(), "grade")
    }

    fn to_sql_checked(
        &self,
        ty: &postgres_types::Type,
        out: &mut bytes::BytesMut,
    ) -> std::result::Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        if !Self::accepts(ty) {
            return Err("Unsupported PostgreSQL type".into());
        }
        self.to_sql(ty, out)
    }
}

struct Image(pub i32);

#[derive(SimpleObject)]
struct PrepareImageUploadResult {
    image: Image,
    upload_url: String,
}

#[Object]
impl Image {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn download_url<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Option<String>> {
        let appdata = ctx.data::<AppData>()?;

        // TODO This should come from configuration
        let bucket = "images";
        let s3 = appdata.s3_pools.get(bucket)
            .ok_or(format!("No {} bucket configured", bucket))?
            .get().await?;

        // TODO I am not a fan of this for at least two reasons
        // - listing objects is slow (at least slower than a db query, I think)
        // - assumes where the image is located
        // climb-pg has an s3_sources table that can be used to know exactly where the sources are
        // located, but this will require some sort of finalization/registration of the source
        // being uploaded
        let prefix = format!("{}/", self.0);
        let listings = s3.list(prefix, Some("/".to_string())).await?;

        let objects = match listings.first() {
            Some(lst) => &lst.contents,
            None => return Ok(None),
        };

        let object = match objects.first() {
            Some(obj) => &obj.key,
            None => return Ok(None),
        };

        Ok(Some(s3.presign_get(object, 300, None).await?))
    }
}

struct Climber(pub i32);

#[Object]
impl Climber {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn first_name<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<String> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT first_name
                FROM climbers
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let value: &str = result.try_get(0)?;

        Ok(value.to_string())
    }

    async fn last_name<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<String> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT last_name
                FROM climbers
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let value: &str = result.try_get(0)?;

        Ok(value.to_string())
    }
}

struct Ascent(pub i32);

#[Object]
impl Ascent {
    async fn id(&self) -> ID {
        self.0.into()
    }

    async fn climb<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT climb_id
                FROM ascents
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let climb_id: i32 = result.try_get(0)?;

        Ok(Climb(climb_id))
    }

    async fn climber<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Climber> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query_one(
                "
                SELECT climber_id
                FROM ascents
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let climber_id: i32 = result.try_get(0)?;

        Ok(Climber(climber_id))
    }

    async fn date_window<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Option<DateInterval>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            // TODO sfackler/rust-postgres-range#18
            .query_one(
                "
                SELECT date_window::TEXT
                FROM ascents
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let value: Option<DateInterval> = result.try_get(0)?;

        Ok(value)
    }

    async fn grades<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Grade>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let value: Vec<Grade> = client
            // TODO since grade doesn't implement binary functions, we must request the text
            // format, when grade _does_ implement these, the ::TEXT is not needed
            .query(
                "
                SELECT grade::TEXT
                FROM ascent_grades
                WHERE ascent_id = $1
                ",
                &[&self.0],
            )
            .await?
            .into_iter()
            .filter_map(|row| {
                let grade_str: String = row.get(0);
                match grade_str.parse::<Grade>() {
                    Ok(grade) => Some(grade),
                    Err(_) => None,
                }
            })
            .collect();

        Ok(value)
    }
}

#[Object]
impl QueryRoot {
    async fn areas<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Area>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT areas.id FROM areas", &[])
            .await?;

        let areas = result
            .into_iter()
            .map(|row| row.try_get(0).map(Area))
            .collect::<Result<Vec<Area>, _>>()?;

        Ok(areas)
    }

    async fn areas_by_parent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area parent")] parent: Option<AreaParentInput>,
    ) -> Result<Vec<Area>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = match parent {
            Some(AreaParentInput::Area(area_id)) => {
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
            }
            None => {
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
            }
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
        #[graphql(desc = "Area id")] id: ID,
    ) -> Result<Area> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM areas WHERE id = $1", &[&id])
            .await?;

        Ok(Area(id))
    }

    async fn ascents<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Ascent>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT ascents.id FROM ascents", &[])
            .await?;

        let ascents = result
            .into_iter()
            .map(|row| row.try_get(0).map(Ascent))
            .collect::<Result<Vec<Ascent>, _>>()?;

        Ok(ascents)
    }

    async fn ascent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Ascent id")] id: ID,
    ) -> Result<Ascent> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM ascents WHERE id = $1", &[&id])
            .await?;

        Ok(Ascent(id))
    }

    async fn climbs<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT climbs.id FROM climbs", &[])
            .await?;

        let climbs = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climb))
            .collect::<Result<Vec<Climb>, _>>()?;

        Ok(climbs)
    }

    async fn climbs_by_parent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParentInput>,
    ) -> Result<Vec<Climb>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = match parent {
            Some(ClimbParentInput::Area(area_id)) => {
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
            }
            Some(ClimbParentInput::Formation(formation_id)) => {
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
            }
            None => {
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
            }
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
        #[graphql(desc = "Climb id")] id: ID,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM climbs WHERE id = $1", &[&id])
            .await?;

        Ok(Climb(id))
    }

    async fn climbers<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Climber>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT climbers.id FROM climbers", &[])
            .await?;

        let climbers = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climber))
            .collect::<Result<Vec<Climber>, _>>()?;

        Ok(climbers)
    }

    async fn climber<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climber id")] id: ID,
    ) -> Result<Climber> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM climbers WHERE id = $1", &[&id])
            .await?;

        Ok(Climber(id))
    }

    async fn formations<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Result<Vec<Formation>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT formations.id FROM formations", &[])
            .await?;

        let formations = result
            .into_iter()
            .map(|row| row.try_get(0).map(Formation))
            .collect::<Result<Vec<Formation>, _>>()?;

        Ok(formations)
    }

    async fn formations_by_parent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation parent")] parent: Option<FormationParentInput>,
    ) -> Result<Vec<Formation>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = match parent {
            Some(FormationParentInput::Area(area_id)) => {
                client
                    .query(
                        "
                        SELECT c.id
                        FROM formations AS c
                        INNER JOIN formation_super_area_closures AS sa ON c.id = sa.formation_id
                        WHERE sa.super_area_id = $1
                        ",
                        &[&area_id],
                    )
                    .await?
            }
            Some(FormationParentInput::Formation(formation_id)) => {
                client
                    .query(
                        "
                        SELECT c.id
                        FROM formations AS c
                        INNER JOIN formation_super_formation_closures AS sf ON c.id = sf.formation_id
                        WHERE sf.super_formation_id = $1
                        ",
                        &[&formation_id],
                    )
                    .await?
            }
            None => {
                client
                    .query(
                        "
                        SELECT c.id
                        FROM formations AS c
                        LEFT JOIN formation_super_area_closures AS sa ON c.id = sa.formation_id
                        LEFT JOIN formation_super_formation_closures AS sf ON c.id = sf.formation_id
                        WHERE sa.super_area_id IS NULL AND sf.super_formation_id IS NULL
                        ",
                        &[],
                    )
                    .await?
            }
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
        #[graphql(desc = "Formation id")] id: ID,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM formations WHERE id = $1", &[&id])
            .await?;

        Ok(Formation(id))
    }

    async fn image<'a>(
        &self,
        ctx: &Context<'a>,
        id: ID,
    ) -> Result<Image> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // Just check for existence
        client
            .query_one("SELECT 1 FROM images WHERE id = $1", &[&id])
            .await?;

        Ok(Image(id))
    }
}

#[derive(OneofObject)]
enum AreaParentInput {
    Area(i32),
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
        #[graphql(desc = "Area parent")] parent: Option<AreaParentInput>,
    ) -> Result<Area> {
        let data = ctx.data::<AppData>()?;
        let mut client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let transaction = client.transaction().await?;

        let area_id = transaction
            .query_one(
                "INSERT INTO areas (name) VALUES ($1) RETURNING id",
                &[&name],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(parent) = parent {
            match parent {
                AreaParentInput::Area(parent_id) => {
                    transaction
                        .execute(
                            "
                            INSERT INTO area_closures (area_id, super_area_id)
                            VALUES ($1, $2)
                            ",
                            &[&area_id, &parent_id],
                        )
                        .await?;
                }
            }
        }

        transaction.commit().await?;

        Ok(Area(area_id))
    }

    async fn add_ascent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb ID")] climb_id: ID,
        #[graphql(desc = "Climber ID")] climber_id: ID,
        #[graphql(desc = "Date window")] date_window: Option<DateInterval>,
    ) -> Result<Ascent> {
        let climb_id: i32 = climb_id.0.parse().map_err(|_| "Invalid ID format")?;
        let climber_id: i32 = climber_id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let ascent_id = client
            .query_one(
                "
                INSERT INTO ascents (climb_id, climber_id, date_window)
                VALUES ($1, $2, $3)
                RETURNING id
                ",
                &[&climb_id, &climber_id, &date_window],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Ascent(ascent_id))
    }

    async fn date_ascent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Ascent ID")] id: ID,
        #[graphql(desc = "Area name")] date_interval: Option<DateInterval>,
    ) -> Result<Ascent> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let ascent_id = client
            .query_one(
                "
                UPDATE ascents
                SET date_window = $1
                WHERE id = $2
                RETURNING id
                ",
                &[&date_interval, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Ascent(ascent_id))
    }

    async fn rename_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: ID,
        #[graphql(desc = "Area name")] name: Option<String>,
    ) -> Result<Area> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let area_id = client
            .query_one(
                "UPDATE areas SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Area(area_id))
    }

    async fn describe_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: ID,
        #[graphql(desc = "Area description")] description: Option<String>,
    ) -> Result<Area> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let area_id = client
            .query_one(
                "UPDATE areas SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Area(area_id))
    }

    async fn move_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: ID,
        #[graphql(desc = "Area parent")] parent: Option<AreaParentInput>,
    ) -> Result<Area> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        if let Some(parent) = parent {
            match parent {
                AreaParentInput::Area(area_id) => {
                    client
                        .execute(
                            "
                            INSERT INTO area_closures (area_id, super_area_id)
                            VALUES ($1, $2)
                            ON CONFLICT (area_id)
                            DO UPDATE SET
                            super_area_id = EXCLUDED.super_area_id
                            ",
                            &[&id, &area_id],
                        )
                        .await?;
                }
            }
        } else {
            client
                .execute(
                    "DELETE FROM area_closures WHERE area_id = $1",
                    &[&id],
                )
                .await?;
        }

        Ok(Area(id))
    }

    async fn describe_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: ID,
        #[graphql(desc = "Formation description")] description: Option<String>,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let formation_id = client
            .query_one(
                "UPDATE formations SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Formation(formation_id))
    }

    async fn remove_area<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Area id")] id: ID,
    ) -> Result<Area> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute("DELETE FROM areas WHERE id = $1", &[&id])
            .await?;

        Ok(Area(id))
    }

    async fn remove_ascent<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Ascent id")] id: ID,
    ) -> Result<Ascent> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute("DELETE FROM ascents WHERE id = $1", &[&id])
            .await?;

        Ok(Ascent(id))
    }

    async fn add_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb name")] name: Option<String>,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParentInput>,
    ) -> Result<Climb> {
        let data = ctx.data::<AppData>()?;
        let mut client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

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
        #[graphql(desc = "Climb id")] id: ID,
        #[graphql(desc = "Climb name")] name: Option<String>,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let id = client
            .query_one(
                "UPDATE climbs SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climb(id))
    }

    async fn describe_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: ID,
        #[graphql(desc = "Climb description")] description: Option<String>,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let climb_id = client
            .query_one(
                "UPDATE climbs SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climb(climb_id))
    }

    async fn move_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: ID,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParentInput>,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let mut client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

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
        } else {
            transaction
                .execute(
                    "DELETE FROM climb_super_formation_closures WHERE climb_id = $1",
                    &[&id],
                )
                .await?;

            transaction
                .execute(
                    "DELETE FROM climb_super_area_closures WHERE climb_id = $1",
                    &[&id],
                )
                .await?;
        }

        transaction.commit().await?;

        Ok(Climb(id))
    }

    async fn add_ascent_grade<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Ascent ID")] id: ID,
        #[graphql(desc = "Grade")] grade: GradeInput,
    ) -> Result<Ascent> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute(
                "
                INSERT INTO ascent_grades (ascent_id, grade)
                VALUES ($1, $2) ON CONFLICT DO NOTHING
                ",
                &[&id, &(grade)],
            )
            .await?;

        Ok(Ascent(id))
    }

    async fn add_climb_grade<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: ID,
        #[graphql(desc = "Grade")] grade: GradeInput,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute(
                "
                INSERT INTO climb_grades (climb_id, grade)
                VALUES ($1, $2) ON CONFLICT DO NOTHING
                ",
                &[&id, &(grade)],
            )
            .await?;

        Ok(Climb(id))
    }

    async fn add_climber<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "First name")] first_name: String,
        #[graphql(desc = "Last name")] last_name: String,
    ) -> Result<Climber> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let id = client
            .query_one(
                "
                INSERT INTO climbers (first_name, last_name)
                VALUES ($1, $2)
                RETURNING id
                ",
                &[&first_name, &last_name],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climber(id))
    }

    async fn remove_ascent_grade<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Ascent ID")] id: ID,
        #[graphql(desc = "Grade")] grade: GradeInput,
    ) -> Result<Ascent> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute(
                "
                DELETE FROM ascent_grades
                WHERE ascent_id = $1 AND grade = $2
                ",
                &[&id, &(grade)],
            )
            .await?;

        Ok(Ascent(id))
    }

    async fn remove_climb_grade<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: ID,
        #[graphql(desc = "Grade")] grade: GradeInput,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute(
                "
                DELETE FROM climb_grades
                WHERE climb_id = $1 AND grade = $2
                ",
                &[&id, &(grade)],
            )
            .await?;

        Ok(Climb(id))
    }

    async fn remove_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: ID,
    ) -> Result<Climb> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute("DELETE FROM climbs WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Climb(id))
    }

    async fn remove_climber<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climber id")] id: ID,
    ) -> Result<Climber> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute("DELETE FROM climbers WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Climber(id))
    }

    async fn add_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation name")] name: Option<String>,
        #[graphql(desc = "Formation location")] location: Option<Coordinate>,
        #[graphql(desc = "Formation parent")] parent: Option<FormationParentInput>,
    ) -> Result<Formation> {
        let data = ctx.data::<AppData>()?;
        let mut client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

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
        #[graphql(desc = "Formation id")] id: ID,
        #[graphql(desc = "Formation name")] name: Option<String>,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

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
        #[graphql(desc = "Formation id")] id: ID,
        #[graphql(desc = "Formation location")] location: Option<Coordinate>,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

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
        #[graphql(desc = "Formation id")] id: ID,
        #[graphql(desc = "Formation parent")] parent: Option<FormationParentInput>,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let mut client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let transaction = client.transaction().await?;

        if let Some(parent) = parent {
            match parent {
                FormationParentInput::Area(area_id) => {
                    transaction
                        .execute(
                            "DELETE FROM formation_super_formation_closures WHERE formation_id = $1",
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
        } else {
            transaction
                .execute(
                    "DELETE FROM formation_super_formation_closures WHERE formation_id = $1",
                    &[&id],
                )
                .await?;

            transaction
                .execute(
                    "DELETE FROM formation_super_area_closures WHERE formation_id = $1",
                    &[&id],
                )
                .await?;
        }
        transaction.commit().await?;

        Ok(Formation(id))
    }

    async fn remove_formation<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Formation id")] id: ID,
    ) -> Result<Formation> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        client
            .execute("DELETE FROM formations WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Formation(id))
    }

    async fn prepare_image_upload<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(
            validator(min_length = 1),
            desc = "Name of image file",
        )] name: String,
    ) -> Result<PrepareImageUploadResult> {
        let appdata = ctx.data::<AppData>()?;

        let mut pg = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // TODO this is configured, it shouldn't be hard-coded..
        let bucket = "images";
        let s3 = appdata.s3_pools.get(bucket)
            .ok_or(format!("No {} bucket configured", bucket))?
            .get().await?;

        let transaction = pg.transaction().await?;

        let image = Image(
            transaction
                .query_one("INSERT INTO images DEFAULT VALUES RETURNING id", &[])
                .await?
                .get::<_, i32>(0),
        );

        let object = format!("{}/{}", image.0, name);
        let upload_url = s3.presign_put(object, 300, None, None).await?;

        // TODO failure to upload an image to the returned url, results in the image row being
        // orphaned.. either need
        // - garbage collection based on a "created-at" column
        // - cleanup when presign url expires and nothing has been uploaded
        transaction.commit().await?;

        Ok(PrepareImageUploadResult { image, upload_url })
    }
}
