use async_graphql::{Context, Object, OneofObject, Result};
use deadpool_postgres::Pool;

use area::Area;
use climb::Climb;
use formation::{Coordinate, Formation};

pub mod area;
pub mod climb;
pub mod formation;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn areas<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] area_id: Option<i32>,
    ) -> Result<Vec<Area>> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        // Just check for existence
        client
            .query_one("SELECT 1 FROM formations WHERE id = $1", &[&id])
            .await?;

        Ok(Formation(id))
    }
}

#[derive(OneofObject)]
enum ClimbParent {
    Area(i32),
    Formation(i32),
}

#[derive(OneofObject)]
enum FormationParent {
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
        let pool = ctx.data::<Pool>()?;
        let mut client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        client
            .execute("DELETE FROM areas WHERE id = $1", &[&id])
            .await?;

        Ok(Area(id))
    }

    async fn add_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb name")] name: Option<String>,
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParent>,
    ) -> Result<Climb> {
        let pool = ctx.data::<Pool>()?;
        let mut client = pool.get().await?;

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
                ClimbParent::Area(area_id) => {
                    transaction
                        .execute(
                            "INSERT INTO climb_super_area_closures (climb_id, super_area_id) VALUES ($1, $2)",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                ClimbParent::Formation(formation_id) => {
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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        #[graphql(desc = "Climb parent")] parent: Option<ClimbParent>,
    ) -> Result<Climb> {
        let pool = ctx.data::<Pool>()?;
        let mut client = pool.get().await?;

        let transaction = client.transaction().await?;

        if let Some(parent) = parent {
            match parent {
                ClimbParent::Area(area_id) => {
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

                ClimbParent::Formation(formation_id) => {
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

    async fn remove_climb<'a>(
        &self,
        ctx: &Context<'a>,
        #[graphql(desc = "Climb id")] id: i32,
    ) -> Result<Climb> {
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        #[graphql(desc = "Formation parent")] parent: Option<FormationParent>,
    ) -> Result<Formation> {
        let pool = ctx.data::<Pool>()?;
        let mut client = pool.get().await?;

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
                FormationParent::Area(area_id) => {
                    transaction
                        .execute(
                            "INSERT INTO formation_super_area_closures (formation_id, super_area_id) VALUES ($1, $2)",
                            &[&id, &area_id],
                        )
                        .await?;
                }

                FormationParent::Formation(formation_id) => {
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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

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
        #[graphql(desc = "Formation parent")] parent: Option<FormationParent>,
    ) -> Result<Formation> {
        let pool = ctx.data::<Pool>()?;
        let mut client = pool.get().await?;

        let transaction = client.transaction().await?;

        if let Some(parent) = parent {
            match parent {
                FormationParent::Area(area_id) => {
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

                FormationParent::Formation(formation_id) => {
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
        let pool = ctx.data::<Pool>()?;
        let client = pool.get().await?;

        client
            .execute("DELETE FROM formations WHERE id = $1", &[&id])
            .await?;

        // TODO Does this make sense?
        Ok(Formation(id))
    }
}
