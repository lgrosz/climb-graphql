use async_graphql::{Context, FieldResult, Error, InputObject, Object, SimpleObject, Enum};

pub struct Area(i32);

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GradeType {
    Vermin,
}

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "GradeInput")]
pub struct Grade {
    #[graphql(name="type")]
    pub grade_type: GradeType,
    pub value: String,
}

#[derive(SimpleObject, InputObject)]
#[graphql(input_name = "CoordinateInput")]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[Object]
impl Area {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn names<'a>(&self, _ctx: &Context<'a>) -> Vec<String> {
        vec![]
    }

    async fn super_area<'a>(&self, _ctx: &Context<'a>) -> Option<Area> {
        None
    }

    async fn sub_areas<'a>(&self, _ctx: &Context<'a>) -> Vec<Area> {
        vec![]
    }

    async fn formations<'a>(&self, _ctx: &Context<'a>) -> Vec<Formation> {
        vec![]
    }

    async fn climbs<'a>(&self, _ctx: &Context<'a>) -> Vec<Climb> {
        vec![]
    }
}

pub struct Climb(i32);

#[Object]
impl Climb {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn names<'a>(&self, _ctx: &Context<'a>) -> Vec<String> {
        vec![]
    }

    async fn grades<'a>(&self, _ctx: &Context<'a>) -> Option<Vec<Grade>> {
        None
    }

    async fn area<'a>(&self, _ctx: &Context<'a>) -> Option<Area> {
        None
    }

    async fn formation<'a>(&self, _ctx: &Context<'a>) -> Option<Formation> {
        None
    }
}

pub struct Formation(i32);

#[Object]
impl Formation {
    async fn id(&self) -> &i32 {
        &self.0
    }

    async fn names<'a>(&self, _ctx: &Context<'a>) -> Vec<String> {
        vec![]
    }

    async fn location<'a>(&self, _ctx: &Context<'a>) -> Option<Coordinate> {
        None
    }

    async fn area<'a>(&self, _ctx: &Context<'a>) -> Option<Area> {
        None
    }

    async fn super_formation<'a>(&self, _ctx: &Context<'a>) -> Option<Formation> {
        None
    }

    async fn sub_formations<'a>(&self, _ctx: &Context<'a>) -> Vec<Formation> {
        vec![]
    }

    async fn climbs<'a>(&self, _ctx: &Context<'a>) -> Vec<Climb> {
        vec![]
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn areas<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Parent area id"
        )]
        _area_id: Option<i32>,
    ) -> FieldResult<Vec<Area>> {
        Err(Error::new("Not implemented"))
    }

    async fn area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Returns the area with the given id"
        )]
        _id: i32,
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn climbs<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Parent area id"
        )]
        _area_id: Option<i32>,
        #[graphql(
            desc = "Parent formation id"
        )]
        _formation_id: Option<i32>
    ) -> FieldResult<Vec<Climb>> {
        Err(Error::new("Not implemented"))
    }

    async fn climb<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Returns climb with given id"
        )]
        _id: i32,
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn formations<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Parent area id"
        )]
        _area_id: Option<i32>,
        #[graphql(
            desc = "Parent formation id"
        )]
        _formation_id: Option<i32>
    ) -> FieldResult<Vec<Formation>> {
        Err(Error::new("Not implemented"))
    }

    async fn formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Returns the formation with given id"
        )]
        _id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
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
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn add_area_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Area id to add name to"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to add"
        )]
        _name: String
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_area_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Area id to remove name from"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to remove"
        )]
        _name: String
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn set_super_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Area id to set 'super area' of"
        )]
        _id: i32,
        #[graphql(
            desc = "Super area id"
        )]
        _super_area_id: i32
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }
    async fn clear_super_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Area id to clear 'super area' of"
        )]
        _id: i32,
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Removes area with given id"
        )]
        _id: i32,
    ) -> FieldResult<Area> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Names to associate with the climb"
        )]
        _names: Option<Vec<String>>,
        #[graphql(
            desc = "Grades to associate with the climb"
        )]
        _grades: Option<Vec<Grade>>,
        #[graphql(
            desc = "Parent area id of the climb"
        )]
        _area_id: Option<i32>,
        #[graphql(
            desc = "Parent formation id of the climb"
        )]
        _formation_id: Option<i32>,
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Climb id to add name to"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to add"
        )]
        _name: String
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Climb id to remove name from"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to remove"
        )]
        _name: String
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_climb_grade<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Climb id to add grade to"
        )]
        _id: i32,
        #[graphql(
            desc = "Grade which to add"
        )]
        _grade: Grade
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb_grade<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(desc = "Climb id to remove grade from")] _id: i32,
        #[graphql(desc = "Grade to remove")] _grade: Grade,
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_climb<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Removes climb with given id"
        )]
        _id: i32,
    ) -> FieldResult<Climb> {
        Err(Error::new("Not implemented"))
    }

    async fn add_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        _names: Option<Vec<String>>,
        _area_id: Option<i32>,
        _super_formation_id: Option<i32>,
        _location: Option<Coordinate>,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn add_formation_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to add name to"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to add"
        )]
        _name: String
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_formation_name<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to remove name from"
        )]
        _id: i32,
        #[graphql(
            desc = "Name which to remove"
        )]
        _name: String
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_location<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to set location of"
        )]
        _id: i32,
        #[graphql(
            desc = "Location of the formation"
        )]
        _location: Coordinate
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn clear_formation_location<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to set location of"
        )]
        _id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to area of"
        )]
        _id: i32,
        #[graphql(
            desc = "Area id"
        )]
        _area_id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn set_formation_super_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to super-formation of"
        )]
        _id: i32,
        #[graphql(
            desc = "Super formation id"
        )]
        _super_formation_id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn clear_formation_area<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to area of"
        )]
        _id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    // TODO This same thing as `clear_formation_area`. Is there a common name I can use to avoid
    // this duplication? Or from an outside view, does it make sense to keep them separate?
    async fn clear_formation_super_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Formation id to super-formation of"
        )]
        _id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }

    async fn remove_formation<'a>(
        &self,
        _ctx: &Context<'a>,
        #[graphql(
            desc = "Removes formation with given id"
        )]
        _id: i32,
    ) -> FieldResult<Formation> {
        Err(Error::new("Not implemented"))
    }
}
