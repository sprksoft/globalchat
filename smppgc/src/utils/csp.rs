use rocket::{http::Header, response::Responder};

pub struct AllowSmIFrame<T>(pub T);
impl<'r, 'o: 'r, T: Responder<'r, 'o>> Responder<'r, 'o> for AllowSmIFrame<T> {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut response = self.0.respond_to(request)?;
        response.set_header(CSPFrameAncestors::SMARTSCHOOL_PLAT);
        Ok(response)
    }
}

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
