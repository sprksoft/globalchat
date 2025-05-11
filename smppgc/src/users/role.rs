use rocket::serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum Role {
    User,
    Mod,
    Admin,
    Owner,
}
impl Role {
    pub fn is_mod(self) -> bool {
        match self {
            Self::User => false,
            Self::Mod | Self::Admin | Self::Owner => true,
        }
    }
}
impl Into<i32> for Role {
    fn into(self) -> i32 {
        match self {
            Self::User => 0,
            Self::Mod => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }
}

impl TryFrom<i32> for Role {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Role::User),
            1 => Ok(Role::Mod),
            2 => Ok(Role::Admin),
            3 => Ok(Role::Owner),
            _ => Err(()),
        }
    }
}
impl TryFrom<String> for Role {
    type Error = ();
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let str: &str = &value;
        Self::try_from(str)
    }
}
impl TryFrom<&str> for Role {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, ()> {
        match value.into() {
            "user" => Ok(Role::User),
            "mod" => Ok(Role::Mod),
            "admin" => Ok(Role::Admin),
            "owner" => Ok(Role::Owner),
            _ => Err(()),
        }
    }
}
