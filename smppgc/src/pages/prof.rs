use crate::{auth::GcAdmin, themes::Theme};
use profanity::{Flags, RuleFlags, Token};
use rocket::get;
use rocket_dyn_templates::{context, Template};

#[get("/prof")]
pub fn prof(_gcadmin: GcAdmin, theme: Theme) -> Template {
    Template::render(
        "pages/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info(), tokeninfo: Token::token_info()},
    )
}
