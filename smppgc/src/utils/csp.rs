use rocket::http::Header;

pub struct CSPFrameAncestors<'a>(pub &'a str);

impl<'a> CSPFrameAncestors<'a> {
    pub const SMARTSCHOOL_PLAT: CSPFrameAncestors<'static> = CSPFrameAncestors("*.smartschool.be");
}
impl<'a> From<CSPFrameAncestors<'a>> for Header<'static> {
    fn from(csp: CSPFrameAncestors) -> Self {
        Header::new(
            "Content-Security-Policy",
            format!("frame-ancestors {};", csp.0),
        )
    }
}
