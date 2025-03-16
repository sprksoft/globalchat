use rocket::http::Header;

pub struct CSPFrameAncestors {
    pub frame_ancestors: String,
}
impl From<CSPFrameAncestors> for Header<'static> {
    fn from(csp: CSPFrameAncestors) -> Self {
        Header::new(
            "Content-Security-Policy",
            format!("frame-ancestors {};", csp.frame_ancestors),
        )
    }
}
