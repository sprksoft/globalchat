use std::borrow::Cow;

mod dutch;
mod english;
mod snowball;

#[derive(Clone, Copy)]
pub enum Stemmer {
    Dutch,
    English,
}
impl Stemmer {
    pub fn stem<'a>(self, input: &'a str) -> Cow<'a, str> {
        let mut env = snowball::SnowballEnv::create(input);
        match self {
            Self::Dutch => {
                dutch::stem(&mut env);
            }
            Self::English => {
                english::stem(&mut env);
            }
        }
        env.get_current()
    }
}
