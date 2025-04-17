use rocket::serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum Role {
    User,
    Mod,
    Admin,
}

impl TryFrom<String> for Role {
    type Error = ();
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let str: &str = &value;
        Self::try_from(str)
    }
}
impl TryFrom<i32> for Role {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Role::User),
            1 => Ok(Role::Mod),
            2 => Ok(Role::Admin),
            _ => Err(()),
        }
    }
}
impl TryFrom<&str> for Role {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        match value.into() {
            "user" => Ok(Role::User),
            "mod" => Ok(Role::Mod),
            "admin" => Ok(Role::Admin),
            _ => Err(()),
        }
    }
}
