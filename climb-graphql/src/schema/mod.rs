use async_graphql::{Context, Error, Object, Result};
use deadpool_postgres::Pool;

use area::Area;
use climb::{Climb, Grade};
use formation::{Coordinate, Formation};

pub mod area;
pub mod climb;
pub mod formation;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn areas<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] _area_id: Option<i32>,
    ) -> Result<Vec<Area>> {
        Err(Error::new("Not implemented"))
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
        _ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] _area_id: Option<i32>,
        #[graphql(desc = "Parent formation id")] _formation_id: Option<i32>,
    ) -> Result<Vec<Climb>> {
        Err(Error::new("Not implemented"))
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
        _ctx: &Context<'a>,
        #[graphql(desc = "Parent area id")] _area_id: Option<i32>,
        #[graphql(desc = "Parent formation id")] _formation_id: Option<i32>,
    ) -> Result<Vec<Formation>> {
        Err(Error::new("Not implemented"))
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

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn add_area<'a>(
        &self,
        _ctx: &Context<'a>,
        _names: Option<Vec<String>>,
        _super_area_id: Option<i32>,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn add_area_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Area id to add name to")] _id: i32,
        #[graphql(desc = "Name which to add")] _name: String,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_area_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Area id to remove name from")] _id: i32,
        #[graphql(desc = "Name which to remove")] _name: String,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn set_super_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Area id to set 'super area' of")] _id: i32,
        #[graphql(desc = "Super area id")] _super_area_id: i32,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }
    async fn clear_super_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Area id to clear 'super area' of")] _id: i32,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Removes area with given id")] _id: i32,
    ) -> Result<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Names to associate with the climb")] _names: Option<Vec<String>>,
        #[graphql(desc = "Grades to associate with the climb")] _grades: Option<Vec<Grade>>,
        #[graphql(desc = "Parent area id of the climb")] _area_id: Option<i32>,
        #[graphql(desc = "Parent formation id of the climb")] _formation_id: Option<i32>,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Climb id to add name to")] _id: i32,
        #[graphql(desc = "Name which to add")] _name: String,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Climb id to remove name from")] _id: i32,
        #[graphql(desc = "Name which to remove")] _name: String,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb_grade<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Climb id to add grade to")] _id: i32,
        #[graphql(desc = "Grade which to add")] _grade: Grade,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb_grade<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Climb id to remove grade from")] _id: i32,
        #[graphql(desc = "Grade to remove")] _grade: Grade,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Removes climb with given id")] _id: i32,
    ) -> Result<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        _names: Option<Vec<String>>,
        _area_id: Option<i32>,
        _super_formation_id: Option<i32>,
        _location: Option<Coordinate>,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn add_formation_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to add name to")] _id: i32,
        #[graphql(desc = "Name which to add")] _name: String,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_formation_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to remove name from")] _id: i32,
        #[graphql(desc = "Name which to remove")] _name: String,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_location<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to set location of")] _id: i32,
        #[graphql(desc = "Location of the formation")] _location: Coordinate,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn clear_formation_location<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to set location of")] _id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to area of")] _id: i32,
        #[graphql(desc = "Area id")] _area_id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_super_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to super-formation of")] _id: i32,
        #[graphql(desc = "Super formation id")] _super_formation_id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn clear_formation_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to area of")] _id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    // TODO This same thing as `clear_formation_area`. Is there a common name I can use to avoid
    // this duplication? Or from an outside view, does it make sense to keep them separate?
    async fn clear_formation_super_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Formation id to super-formation of")] _id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Removes formation with given id")] _id: i32,
    ) -> Result<Formation> {
        Err(Error::new("Not implemented"))
    }
}
