use std::borrow::Cow;

mod dutch;
mod english;
mod snowball;

pub fn stem<'a>(input: &'a str) -> Cow<'a, str> {
    let mut env = snowball::SnowballEnv::create(input);
    english::stem(&mut env);
    // if env.current == input {
    //     english::stem(&mut env);
    // }
    env.get_current()
}
