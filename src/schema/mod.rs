use std::fmt::Display;

use async_graphql::{Context, Object, OneofObject, Result, SimpleObject, ID};

use grade::GradeInput;
use postgres_types::ToSql;
use mutation_root::image::ImageMutationRoot;
use mutation_root::topo::TopoMutationRoot;
use types::climb::Climb;
use types::formation::{Coordinate, Formation};
use types::topo::Topo;

use crate::AppData;

pub mod grade;
pub mod fontainebleau_grade;
pub mod vermin_grade;
pub mod yosemite_decimal_grade;
pub mod mutation_root;
pub mod types;

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

    async fn alt(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Option<String>> {
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
                SELECT alt
                FROM images
                WHERE id = $1
                ",
                &[&self.0],
            )
            .await?;

        let value: Option<&str> = result.try_get(0)?;

        Ok(value.map(|s| s.to_string()))
    }

    async fn download_url(
        &self,
        ctx: &Context<'_>,
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

    async fn formations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Formation>> {
        let appdata = ctx.data::<AppData>()?;
        let client = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query(
                "
                SELECT formation_id
                FROM formations_in_image
                WHERE image_id = $1
                ", &[&self.0]
            ).await?;

        let formations = result
            .into_iter()
            .map(|row| row.try_get(0).map(Formation))
            .collect::<Result<Vec<Formation>, _>>()?;

        Ok(formations)
    }
}

#[Object]
impl QueryRoot {
    async fn regions(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<types::region::Region>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT id FROM climb.regions", &[])
            .await?;

        let regions = result
            .into_iter()
            .map(|row| row.try_get(0).map(types::region::Region))
            .collect::<Result<Vec<types::region::Region>, _>>()?;

        Ok(regions)
    }

    async fn region(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Region id")] id: ID,
    ) -> Result<types::region::Region> {
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
            .query_one("SELECT 1 FROM climb.regions WHERE id = $1", &[&id])
            .await?;

        Ok(types::region::Region(id))
    }

    async fn crags(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<types::crag::Crag>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query(
                "
                SELECT id
                FROM climb.crags
                WHERE region_id IS NULL
                ", &[])
            .await?;

        let crags = result
            .into_iter()
            .map(|row| row.try_get(0).map(types::crag::Crag))
            .collect::<Result<Vec<types::crag::Crag>, _>>()?;

        Ok(crags)
    }

    async fn crag(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Crag id")] id: ID,
    ) -> Result<types::crag::Crag> {
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
            .query_one("SELECT 1 FROM climb.crags WHERE id = $1", &[&id])
            .await?;

        Ok(types::crag::Crag(id))
    }

    async fn sector(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Sector id")] id: ID,
    ) -> Result<types::sector::Sector> {
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
            .query_one("SELECT 1 FROM climb.sectors WHERE id = $1", &[&id])
            .await?;

        Ok(types::sector::Sector(id))
    }

    async fn climbs(
        &self,
        ctx: &Context<'_>,
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

    async fn climbs_by_parent(
        &self,
        ctx: &Context<'_>,
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

    async fn climb(
        &self,
        ctx: &Context<'_>,
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

    async fn formations(
        &self,
        ctx: &Context<'_>,
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

    async fn formations_by_parent(
        &self,
        ctx: &Context<'_>,
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

    async fn formation(
        &self,
        ctx: &Context<'_>,
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

    async fn images(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Image>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT images.id FROM images", &[])
            .await?;

        let images = result
            .into_iter()
            .map(|row| row.try_get(0).map(Image))
            .collect::<Result<Vec<Image>, _>>()?;

        Ok(images)
    }

    async fn image(
        &self,
        ctx: &Context<'_>,
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

    async fn topo(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Topo> {
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
            .query_one("SELECT 1 FROM topos WHERE id = $1", &[&id])
            .await?;

        Ok(Topo(id))
    }

    async fn topos_by_formation(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Vec<Topo>> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("
                SELECT t.id FROM topos AS t
                    INNER JOIN topo_image_features AS tif ON t.id = tif.topo_id
                    INNER JOIN images AS i ON i.id = tif.image_id
                    INNER JOIN formations_in_image AS fii ON fii.image_id = i.id
                WHERE fii.formation_id = $1;
                ", &[&id])
            .await?;

        let topos = result
            .into_iter()
            .map(|row| row.try_get(0).map(Topo))
            .collect::<Result<Vec<Topo>, _>>()?;

        Ok(topos)
    }
}

#[derive(OneofObject)]
enum ClimbParentInput {
    Formation(i32),
}

#[derive(OneofObject)]
enum FormationParentInput {
    Formation(i32),
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn describe_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn add_climb(
        &self,
        ctx: &Context<'_>,
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

    async fn rename_climb(
        &self,
        ctx: &Context<'_>,
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

    async fn describe_climb(
        &self,
        ctx: &Context<'_>,
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

    async fn move_climb(
        &self,
        ctx: &Context<'_>,
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

    async fn add_climb_grade(
        &self,
        ctx: &Context<'_>,
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

    async fn remove_climb_grade(
        &self,
        ctx: &Context<'_>,
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

    async fn remove_climb(
        &self,
        ctx: &Context<'_>,
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

    async fn add_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn rename_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn relocate_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn move_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn remove_formation(
        &self,
        ctx: &Context<'_>,
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

    async fn add_topo(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Topo title")] title: Option<String>,
        #[graphql(desc = "Topo width")] width: f64,
        #[graphql(desc = "Topo height")] height: f64,
    ) -> Result<Topo> {
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
                INSERT INTO topos (title, width, height)
                VALUES ($1, $2, $3)
                RETURNING id
                ",
                &[&title, &width, &height],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Topo(id))
    }

    async fn prepare_image_upload(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            validator(min_length = 1),
            desc = "Name of image file",
        )] name: String,
        #[graphql(
            validator(min_length = 1),
            desc = "Alternative text",
        )] alt: Option<String>,
        #[graphql(
            desc = "IDs of formations in this image",
        )] formation_ids: Option<Vec<ID>>,
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
                .query_one("INSERT INTO images (alt) VALUES ($1) RETURNING id", &[&alt])
                .await?
                .get::<_, i32>(0),
        );

        if let Some(ids) = &formation_ids {
            for formation_id in ids {
                let formation_id: i32 = formation_id
                    .as_str()
                    .parse()
                    .map_err(|_| "Invalid formation ID".to_string())?;
                transaction.execute(
                    "INSERT INTO formations_in_image (formation_id, image_id) VALUES ($1, $2)",
                    &[&formation_id, &image.0],
                ).await?;
            }
        }

        let object = format!("{}/{}", image.0, name);
        let upload_url = s3.presign_put(object, 300, None, None).await?;

        // TODO failure to upload an image to the returned url, results in the image row being
        // orphaned.. either need
        // - garbage collection based on a "created-at" column
        // - cleanup when presign url expires and nothing has been uploaded
        transaction.commit().await?;

        Ok(PrepareImageUploadResult { image, upload_url })
    }

    async fn image(
        &self,
        #[graphql(desc = "ID of image")]
        id: ID,
    ) -> ImageMutationRoot {
        ImageMutationRoot { id }
    }

    async fn topo(
        &self,
        #[graphql(desc = "ID of topo")]
        id: ID,
    ) -> TopoMutationRoot {
        TopoMutationRoot { id }
    }
}
