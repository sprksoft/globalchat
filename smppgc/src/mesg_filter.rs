use std::time::SystemTime;

use crate::chat::Message;

pub enum Cmd {
    BanWord(Box<str>),
    KickMe { hard: bool },
    Invalid,
}

pub enum FilterResult {
    Message(Message),
    Cmd(Message, Cmd),
    Invalid,
}

fn quotes_of_the_minute() -> (&'static str, &'static str) {
    let minutes = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
        / 60;
    if minutes % 2 == 0 {
        ("\"", "'")
    } else {
        ("'", "\"")
    }
}

fn parse_admin_cmd(str: &str) -> Cmd {
    let (quote_start, quote_end) = quotes_of_the_minute();
    let banword_cmd = "/banword ";
    let kickme_cmd = "/kickme";
    let kickmehard_cmd = "/kickme hard";
    if str.starts_with(banword_cmd) && str[banword_cmd.len()..].starts_with(quote_start) {
        let Some(end) = str.find(quote_end) else {
            return Cmd::Invalid;
        };
        let word = str[banword_cmd.len() + 1..end].to_lowercase();
        Cmd::BanWord(word.into())
    } else if str == kickme_cmd {
        Cmd::KickMe { hard: false }
    } else if str == kickmehard_cmd {
        Cmd::KickMe { hard: true }
    } else {
        Cmd::Invalid
    }
}

fn parse_cmd(str: &str) -> Option<Cmd> {
    let admin_prefix = "%admin";
    if str.starts_with(admin_prefix) {
        let str = &str[admin_prefix.len()..].trim();
        return Some(parse_admin_cmd(str));
    }
    None
}

pub fn filter(mut mesg: Message) -> FilterResult {
    if !mesg.is_valid() {
        return FilterResult::Invalid;
    };
    let content = mesg.content.as_ref().trim();
    if let Some(cmd) = parse_cmd(content) {
        return FilterResult::Cmd(mesg, cmd);
    }
    let word = ['k', 'y', 's'];
    let is_kys = content.len() >= 3
        && content
            .chars()
            .filter(|char| !char.is_whitespace())
            .zip(word)
            .all(|(char, word_char)| char.to_lowercase().next() == Some(word_char));

    if is_kys {
        mesg.content = "Kiss me pwees".into();
    }

    FilterResult::Message(mesg)
}
