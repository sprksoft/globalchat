use crate::models::{Role, UserId};

pub struct User {
    role: Role,
    id: UserId,
    irl_name: Box<str>,
}
impl User {
    pub fn new(id: UserId, role: Role, irl_name: Box<str>) -> Self {
        Self { role, id, irl_name }
    }

    pub fn id(&self) -> UserId {
        self.id
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn irl_name(&self) -> &str {
        &self.irl_name
    }
}
