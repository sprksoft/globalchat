pub fn clean_line<'a>(str: &'a str) -> Option<&'a str> {
    let str = str.trim();
    if str.len() == 0 {
        return None;
    }

    Some(str)
}
