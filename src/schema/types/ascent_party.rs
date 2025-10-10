use async_graphql::Object;

use super::climber::Climber;

pub struct AscentParty {
    pub complete: bool,
    pub member_ids: Vec<i32>
}

#[Object]
impl AscentParty {
    async fn complete(&self) -> bool {
        self.complete
    }

    async fn members(&self) -> Vec<Climber> {
        self.member_ids.iter().copied().map(Climber).collect()
    }
}
