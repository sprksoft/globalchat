use crate::{themes::Theme, users::AdminUser};
use profanity::{Flags, RuleFlags, Token};
use rocket::get;
use rocket_dyn_templates::{context, Template};

#[get("/prof")]
pub fn prof(_ses: AdminUser, theme: Theme) -> Template {
    Template::render(
        "pages/prof",
        context! {theme_css:theme.css(), flagsinfo: RuleFlags::flags_info(), tokeninfo: Token::token_info()},
    )
}
