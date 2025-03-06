use lazy_static::lazy_static;

lazy_static! {
    pub static ref VERSION_INT: u16 = {
        let year: usize = env!("CARGO_PKG_VERSION_MAJOR")
            .parse()
            .expect("Major version number can't be parsed into a usize");
        let month: usize = env!("CARGO_PKG_VERSION_MINOR")
            .parse()
            .expect("Minor version number can't be parsed into a usize");

        let serial: usize = env!("CARGO_PKG_VERSION_PATCH")
            .parse()
            .expect("Patch version number can't be parsed into a usize");

        const MAX_SERIALS_PER_MONTH: usize = 20;
        #[cfg(debug_assertions)]
        if serial > MAX_SERIALS_PER_MONTH {
            panic!("Version overflow");
        }
        ((year - 2024) * (12 * MAX_SERIALS_PER_MONTH) + month * MAX_SERIALS_PER_MONTH + serial)
            as u16
    };
}
