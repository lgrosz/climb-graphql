use async_graphql::{InputObject, ID};

#[derive(InputObject)]
pub struct AscentPartyInput {
    pub complete: bool,
    pub member_ids: Vec<ID>,
}
