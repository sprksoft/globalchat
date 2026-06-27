use std::borrow::Cow;

use rocket::{
    form::FromFormField,
    serde::{Deserialize, Serialize},
};

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
        self >= Self::Mod
    }
    pub fn is_admin(self) -> bool {
        self >= Self::Admin
    }
    pub fn from_i32(num: i32) -> Option<Self> {
        match num {
            0 => Some(Self::User),
            1 => Some(Self::Mod),
            2 => Some(Self::Admin),
            3 => Some(Self::Owner),
            _ => None,
        }
    }
    pub fn to_i32(self) -> i32 {
        self.to_u8() as i32
    }
    pub fn to_u8(self) -> u8 {
        match self {
            Self::User => 0,
            Self::Mod => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }
    pub fn to_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Mod => "mod",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }
}

impl PartialOrd for Role {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_i32().partial_cmp(&other.to_i32())
    }
}
impl TryFrom<i32> for Role {
    type Error = ();
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::from_i32(value).ok_or(())
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
impl<'v> FromFormField<'v> for Role {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Self::try_from(field.value).map_err(|_| &[Cow::Borrowed("invalid role value")])?)
    }
}
