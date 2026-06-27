use std::ops::Deref;

pub struct ClaimedName(Box<str>);
impl ClaimedName {
    pub fn new(name: impl Into<Box<str>>) -> Self {
        Self(name.into())
    }
}
impl Deref for ClaimedName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Into<String> for ClaimedName {
    fn into(self) -> String {
        self.0.into()
    }
}
