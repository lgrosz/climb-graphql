use std::fmt::Display;
use std::io::Read;

use async_graphql::{Context, Object, OneofObject, Result, Upload, ID};

use grade::GradeInput;
use postgres_types::ToSql;
use mutation_root::image::ImageMutationRoot;
use mutation_root::topo::TopoMutationRoot;
use scalars::date_range::DateRange;
use types::ascent::Ascent;
use types::ascent_party_input::AscentPartyInput;
use types::climb::Climb;
use types::climber::Climber;
use types::crag::Crag;
use types::formation::{Coordinate, Formation};
use types::region::Region;
use types::sector::Sector;
use types::topo::Topo;

use crate::{AppData, WithAppData};

pub mod grade;
pub mod fontainebleau_grade;
pub mod vermin_grade;
pub mod yosemite_decimal_grade;
pub mod mutation_root;
pub mod scalars;
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
                FROM media.images
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
                FROM climb.formations_in_image
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
    async fn ascent(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Ascent ID")]
        id: ID,
    ) -> Result<Ascent> {
        let client = ctx.db_client().await?;
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let row = client.query_one("SELECT id from climb.ascents WHERE id = $1", &[&id]).await?;
        Ok(row.try_get(0).map(Ascent)?)
    }

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

    async fn climbers(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Climber>> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let result = client
            .query("SELECT climbers.id FROM climb.climbers", &[])
            .await?;

        let climbers = result
            .into_iter()
            .map(|row| row.try_get(0).map(Climber))
            .collect::<Result<Vec<Climber>, _>>()?;

        Ok(climbers)
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
            .query(
                "
                SELECT climbs.id
                FROM climb.climbs
                WHERE region_id IS NULL
                    AND crag_id IS NULL
                    AND sector_id IS NULL
                    AND formation_id IS NULL
                ", &[])
            .await?;

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
            .query_one("SELECT 1 FROM climb.climbs WHERE id = $1", &[&id])
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
            .query(
                "
                SELECT formations.id
                FROM climb.formations
                WHERE region_id IS NULL
                    AND crag_id IS NULL
                    AND sector_id IS NULL
                ", &[])
            .await?;

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
            .query_one("SELECT 1 FROM climb.formations WHERE id = $1", &[&id])
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
            .query("SELECT images.id FROM media.images", &[])
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
            .query_one("SELECT 1 FROM media.images WHERE id = $1", &[&id])
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
            .query_one("SELECT 1 FROM topo.topos WHERE id = $1", &[&id])
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
                SELECT t.id FROM topo.topos AS t
                    INNER JOIN topo.image_features AS tif ON t.id = tif.topo_id
                    INNER JOIN media.images AS i ON i.id = tif.image_id
                    INNER JOIN climb.formations_in_image AS fii ON fii.image_id = i.id
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
    Region(ID),
    Crag(ID),
    Sector(ID),
    Formation(ID),
}

#[derive(OneofObject)]
enum FormationParentInput {
    Region(ID),
    Crag(ID),
    Sector(ID),
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn add_ascent(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Climb ID")]
        climb_id: ID,
        #[graphql(desc = "Ascent date window")]
        date_window: Option<DateRange>,
        #[graphql(desc = "Ascent party")]
        party: AscentPartyInput,
        #[graphql(desc = "Whether this is a first ascent")]
        first_ascent: bool,
        #[graphql(desc = "Whether this ascent is verified")]
        verified: bool,
    ) -> Result<Ascent> {
        let mut client = ctx.db_client().await?;
        let tx = client.transaction().await?;

        let climb_id = climb_id.parse::<i32>().map_err(|_| "Invalid climb ID")?;
        let ascent_window = date_window.as_ref().map(|r| r.to_string());

        let row = tx.query_one(
            "
            INSERT INTO climb.ascents
                (climb_id, ascent_window, ascent_duration, first_ascent, members_complete, verified)
            VALUES
                ($1, COALESCE($2, NULL)::DATERANGE, NULL, $3, $4, $5)
            RETURNING id
            ",
            &[&climb_id, &ascent_window, &first_ascent, &party.complete, &verified],
        ).await?;

        let ascent_id: i32 = row.get(0);

        for member_id in &party.member_ids {
            let climber_id: i32 = member_id.parse().map_err(|_| "Invalid climber ID")?;
            tx.execute(
                "
                INSERT INTO climb.ascent_members (ascent_id, climber_id)
                VALUES ($1, $2)
                ",
                &[&ascent_id, &climber_id],
            ).await?;
        }

        tx.commit().await?;
        Ok(Ascent(ascent_id))
    }

    async fn add_crag(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Crag name")]
        name: Option<String>,
        #[graphql(desc = "Crag description")]
        description: Option<String>,
        #[graphql(desc = "Region ID")]
        region_id: Option<ID>,
    ) -> Result<Crag> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let region_id = region_id
            .map(|s| s.parse::<i32>())
            .transpose()?;

        let id = client
            .query_one(
                "
                INSERT INTO climb.crags
                    (name, description, region_id)
                VALUES ($1, $2, $3)
                RETURNING id
                ",
                &[&name, &description, &region_id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Crag(id))
    }

    async fn add_region(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Region name")]
        name: Option<String>,
        #[graphql(desc = "Region description")]
        description: Option<String>,
    ) -> Result<Region> {
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
                INSERT INTO climb.regions
                    (name, description)
                VALUES ($1, $2)
                RETURNING id
                ",
                &[&name, &description],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Region(id))
    }

    async fn add_sector(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Sector name")]
        name: Option<String>,
        #[graphql(desc = "Sector description")]
        description: Option<String>,
        #[graphql(desc = "Crag ID")]
        crag_id: ID,
    ) -> Result<Sector> {
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let crag_id = crag_id.parse::<i32>()?;

        let id = client
            .query_one(
                "
                INSERT INTO climb.sectors
                    (name, description, crag_id)
                VALUES ($1, $2, $3)
                RETURNING id
                ",
                &[&name, &description, &crag_id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Sector(id))
    }

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
                "UPDATE climb.formations SET description = $1 WHERE id = $2 RETURNING id",
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
        #[graphql(desc = "Climb description")] description: Option<String>,
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
                "
                INSERT INTO
                climb.climbs (name, description)
                VALUES ($1, $2)
                RETURNING id
                ",
                &[&name, &description],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(parent) = parent {
            let (column, id_str) = match parent {
                ClimbParentInput::Region(region_id) => ("region_id", region_id),
                ClimbParentInput::Crag(crag_id) => ("crag_id", crag_id),
                ClimbParentInput::Sector(sector_id) => ("sector_id", sector_id),
                ClimbParentInput::Formation(formation_id) => ("formation_id", formation_id),
            };

            let sql = format!("UPDATE climb.climbs SET {} = $2 WHERE id = $1", column);

            transaction
                .execute(&sql, &[&id, &id_str.parse::<i32>()?])
                .await?;
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
                "UPDATE climb.climbs SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climb(id))
    }

    async fn rename_crag(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Crag id")] id: ID,
        #[graphql(desc = "Crag name")] name: Option<String>,
    ) -> Result<Crag> {
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
                "UPDATE climb.crags SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Crag(id))
    }

    async fn rename_region(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Region id")] id: ID,
        #[graphql(desc = "Region name")] name: Option<String>,
    ) -> Result<Region> {
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
                "UPDATE climb.regions SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Region(id))
    }

    async fn rename_sector(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Sector id")] id: ID,
        #[graphql(desc = "Sector name")] name: Option<String>,
    ) -> Result<Sector> {
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
                "UPDATE climb.sectors SET name = $1 WHERE id = $2 RETURNING id",
                &[&name, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Sector(id))
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
                "UPDATE climb.climbs SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Climb(climb_id))
    }

    async fn describe_crag(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Crag id")] id: ID,
        #[graphql(desc = "Crag description")] description: Option<String>,
    ) -> Result<Crag> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let crag_id = client
            .query_one(
                "UPDATE climb.crags SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Crag(crag_id))
    }

    async fn describe_region(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Region id")] id: ID,
        #[graphql(desc = "Region description")] description: Option<String>,
    ) -> Result<Region> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let region_id = client
            .query_one(
                "UPDATE climb.regions SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Region(region_id))
    }

    async fn describe_sector(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Sector id")] id: ID,
        #[graphql(desc = "Sector description")] description: Option<String>,
    ) -> Result<Sector> {
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;
        let data = ctx.data::<AppData>()?;
        let client = match &data.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        let sector_id = client
            .query_one(
                "UPDATE climb.sectors SET description = $1 WHERE id = $2 RETURNING id",
                &[&description, &id],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Sector(sector_id))
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

        transaction
            .execute(
                "
                UPDATE climb.climbs
                SET region_id = NULL,
                    crag_id = NULL,
                    sector_id = NULL,
                    formation_id = NULL
                WHERE id = $1
                ",
                &[&id],
            )
            .await?;

        if let Some(parent) = parent {
            let (column, id_str) = match parent {
                ClimbParentInput::Region(region_id) => ("region_id", region_id),
                ClimbParentInput::Crag(crag_id) => ("crag_id", crag_id),
                ClimbParentInput::Sector(sector_id) => ("sector_id", sector_id),
                ClimbParentInput::Formation(formation_id) => ("formation_id", formation_id),
            };

            let sql = format!("UPDATE climb.climbs SET {} = $2 WHERE id = $1", column);

            transaction
                .execute(&sql, &[&id, &id_str.parse::<i32>()?])
                .await?;
        }

        transaction.commit().await?;

        Ok(Climb(id))
    }

    async fn move_crag(
        &self,
        ctx: &Context<'_>,
        id: ID,
        region_id: Option<ID>
    ) -> Result<Crag> {
        let db_client = ctx.db_client().await?;

        let id: i32 = id.parse()?;
        let region_id: Option<i32> = region_id.map(|r| r.parse()).transpose()?;

        let _ = db_client.execute(
            "
            UPDATE climb.crags SET region_id = $1 WHERE id = $2
            ", &[&region_id, &id]).await;

        Ok(Crag(id))
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
                UPDATE climb.climbs
                SET grades = array_append(grades, $2)
                WHERE id = $1 AND NOT ($2 = ANY(grades))
                ",
                &[&id, &(grade)],
            )
            .await?;

        Ok(Climb(id))
    }

    async fn remove_ascents(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Ascent IDs")] ids: Vec<ID>,
    ) -> Result<Vec<Ascent>> {
        let ids: Vec<i32> = ids.into_iter()
            .map(|s| s.parse().map_err(|_| "Invalid ID format"))
            .collect::<Result<_, _>>()?;

        let client = ctx.db_client().await?;

        let rows = client
            .query(
                "
                DELETE FROM climb.ascents
                WHERE id = ANY($1)
                RETURNING id
                ",
                &[&ids],
            )
            .await?;

        let ascents = rows.into_iter().map(|row| {
            let id: i32 = row.get(0);
            Ascent(id)
        }).collect();

        Ok(ascents)
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
                UPDATE climb.climbs
                SET grades = array_remove(grades, $2)
                WHERE id = $1
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
            .execute("DELETE FROM climb.climbs WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Climb(id))
    }

    async fn add_formation(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Formation name")] name: Option<String>,
        #[graphql(desc = "Formation description")] description: Option<String>,
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
                "
                INSERT INTO
                    climb.formations (name, description, location)
                    VALUES ($1, $2, $3)
                    RETURNING id",
                &[&name, &description, &point],
            )
            .await?
            .get::<_, i32>(0);

        if let Some(parent) = parent {
            let (column, id_str) = match parent {
                FormationParentInput::Region(region_id) => ("region_id", region_id),
                FormationParentInput::Crag(crag_id) => ("crag_id", crag_id),
                FormationParentInput::Sector(sector_id) => ("sector_id", sector_id),
            };

            let sql = format!("UPDATE climb.formations SET {} = $2 WHERE id = $1", column);

            transaction
                .execute(&sql, &[&id, &id_str.parse::<i32>()?])
                .await?;
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
                "UPDATE climb.formations SET name = $1 WHERE id = $2 RETURNING id",
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
                "UPDATE climb.formations SET location = $1 WHERE id = $2 RETURNING id",
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

        transaction
            .execute(
                "
                UPDATE climb.formations
                SET region_id = NULL,
                    crag_id = NULL,
                    sector_id = NULL
                WHERE id = $1
                ",
                &[&id],
            )
            .await?;

        if let Some(parent) = parent {
            let (column, id_str) = match parent {
                FormationParentInput::Region(region_id) => ("region_id", region_id),
                FormationParentInput::Crag(crag_id) => ("crag_id", crag_id),
                FormationParentInput::Sector(sector_id) => ("sector_id", sector_id),
            };

            let sql = format!("UPDATE climb.formations SET {} = $2 WHERE id = $1", column);

            transaction
                .execute(&sql, &[&id, &id_str.parse::<i32>()?])
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
            .execute("DELETE FROM climb.formations WHERE id = $1", &[&id])
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
                INSERT INTO topo.topos (title, width, height)
                VALUES ($1, $2, $3)
                RETURNING id
                ",
                &[&title, &width, &height],
            )
            .await?
            .get::<_, i32>(0);

        Ok(Topo(id))
    }

    async fn upload_image(
        &self,
        ctx: &Context<'_>,
        #[graphql(
            desc = "Image upload",
        )] image: Upload,
        #[graphql(
            validator(min_length = 1),
            desc = "Alternative text",
        )] alt: Option<String>,
        #[graphql(
            desc = "IDs of formations in this image",
        )] formation_ids: Option<Vec<ID>>,
    ) -> Result<Image> {
        let appdata = ctx.data::<AppData>()?;

        let mut pg = match &appdata.pg_pool {
            Some(pool) => pool.get().await?,
            None => {
                return Err("Database connection is not available".into());
            }
        };

        // TODO this should be configured, I can imagine something like bhbouldering-images-prod
        let bucket_name = "images";
        let bucket = appdata.s3_pools.get(bucket_name)
            .ok_or(format!("No {} bucket configured", bucket_name))?
            .get().await?;

        let transaction = pg.transaction().await?;

        let image_id =
            transaction
                .query_one("INSERT INTO media.images (alt) VALUES ($1) RETURNING id", &[&alt])
                .await?
                .get::<_, i32>(0);

        let filename = image.value(ctx).unwrap().filename;
        let object = format!("{}/{}", image_id, filename);

        // TODO This blocks
        let mut file = image.value(ctx).unwrap().content;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let bytes = buf.as_slice();

        bucket.put_object(object, bytes).await?;

        if let Some(ids) = &formation_ids {
            for formation_id in ids {
                let formation_id: i32 = formation_id
                    .as_str()
                    .parse()
                    .map_err(|_| "Invalid formation ID".to_string())?;
                transaction.execute(
                    "INSERT INTO climb.formations_in_image (formation_id, image_id) VALUES ($1, $2)",
                    &[&formation_id, &image_id],
                ).await?;
            }
        }

        transaction.commit().await?;

        Ok(Image(image_id))
    }

    async fn verify_ascent(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "ID of ascent")]
        id: ID,
        #[graphql(desc = "Whether or not the ascent is verified")]
        is_verified: bool,
    ) -> Result<Ascent> {
        let client = ctx.db_client().await?;
        let id: i32 = id.0.parse().map_err(|_| "Invalid ID format")?;

        let row = client
            .query_one(
                "
                UPDATE climb.ascents
                SET verified = $1
                WHERE id = $2
                RETURNING id
                ",
                &[&is_verified, &id],
            )
            .await
            .map_err(|e| format!("Failed to update ascent: {e}"))?;

        Ok(row.try_get(0).map(Ascent)?)
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
