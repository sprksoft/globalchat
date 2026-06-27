#[derive(Debug)]
pub struct Ban {
    pub reason: Option<String>,
    pub expiration_time: i32,
}
impl Ban {
    pub fn reason(&self) -> &str {
        match self.reason.as_ref() {
            Some(r) => &*r,
            None => "",
        }
    }

    pub fn into_close_frame(&self) -> rocket_ws::frame::CloseFrame<'static> {
        rocket_ws::frame::CloseFrame {
            code: rocket_ws::frame::CloseCode::Normal,
            reason: format!("err_banned:{}:{}", self.expiration_time, self.reason()).into(),
        }
    }
}
