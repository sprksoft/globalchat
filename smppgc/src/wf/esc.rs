macro_rules! escaping {
    ($($esc:literal:$char:literal),*) => {
        pub fn escape(str: &str, buffer: &mut String) {
            for char in str.chars() {
                match char {
                    $(
                        $char=>{buffer.push('\\'); buffer.push($esc)}
                    ),*
                    _=> { buffer.push(char)},
                }
            }
        }

        pub fn unescape(str: &str) -> String {
            let mut string = String::with_capacity(str.len());
            let mut esc = false;
            for char in str.chars() {
                if char == '\\' {
                    esc = true;
                    continue;
                }
                if esc {
                    match char {
                        $(
                            $esc=>{ string.push($char); }
                        ),*
                        _=>{},
                    }
                } else {
                    string.push(char);
                }
            }
            string
        }
    };
}

escaping!(
    '\\':'\\',
    'n':'\n'
);
