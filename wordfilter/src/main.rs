use std::{
    fs,
    io::{self, stdin, Read},
};

use ansii::*;
use clap::Parser;
use wordfilter::{CheckResult, Word, WordFilter};

mod ansii;
mod cleaning;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// good data to add to the filter.
    #[arg(long, short = 'g')]
    add_good: Vec<String>,

    /// Check the data in the file against the filter (use - for stdin)
    #[arg(long)]
    check: Option<String>,

    /// Bad data to add to the filter interactively.
    #[arg(long, short = 'b')]
    bad: Vec<String>,

    /// Optional output file of the resulting filter
    /// When the output file ends in .txt write the filter as text
    #[arg(long, short = 'o')]
    output: Option<String>,

    /// list less than 3 letter words in the filter
    #[arg(long)]
    short_words: bool,

    /// Filter files to merge into the final result
    #[arg(long, short = 'f')]
    filter: Vec<String>,
}

fn get_input_data(path: &str) -> io::Result<String> {
    if path == "-" {
        println!("Reading from stdin...");
        let mut str = String::new();
        io::stdin().read_to_string(&mut str).unwrap();
        Ok(str)
    } else {
        let can_path = std::fs::canonicalize(&path)?;
        let str = fs::read_to_string(can_path)?;
        Ok(str)
    }
}

fn ask_yn(question: &str) -> bool {
    loop {
        println!("{} >", question);
        let mut str = String::new();
        io::stdin().read_line(&mut str).unwrap();
        let str = str.trim();
        if str == "y" || str == "yes" || str == "g" || str == "good" {
            return true;
        } else if str == "n" || str == "no" || str == "b" || str == "bad" {
            return false;
        }
    }
}

fn add_good_file(filter: &mut WordFilter, path: &str) {
    println!("ADDING GOOD WORDS FROM: {}", path);
    for line in get_input_data(path)
        .unwrap()
        .lines()
        .map(|l| cleaning::clean_line(l))
    {
        let Some(line) = line else {
            continue;
        };
        match filter.check(line) {
            CheckResult::Unknown(_) => filter.train_good(line),
            CheckResult::Bad(word) => {
                println!("'{}' ({}) {COLOR_RED}bad{RESET}", line, word.root());
                if ask_yn("mark as good?") {
                    filter.train_word(word.str(), true);
                }
            }
            _ => {}
        }
    }
}

#[inline]
fn interactive_check(
    filter: &mut WordFilter,
    line: &str,
    good: &mut usize,
    bad: &mut Vec<Word>,
    unknown: &mut Vec<Word>,
    count: &mut usize,
) {
    let result = filter.check(&line);
    *count += 1;
    match result {
        CheckResult::Good => {
            println!("{COLOR_GREEN}good{RESET}");
            *good += 1;
        }
        CheckResult::Bad(word) => {
            println!("{COLOR_RED}bad: {} {RESET}", word.str());
            bad.push(word);
        }
        CheckResult::Unknown(word) => {
            println!("{COLOR_GRAY}unknown: {}{RESET}", word.str());
            unknown.push(word);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let mut filter = WordFilter::default();
    for filter_path in cli.filter {
        filter.merge(WordFilter::from_string(
            &std::fs::read_to_string(&filter_path).expect("Could not read filter file"),
        ));
    }

    if let Some(check_file) = cli.check {
        let mut count = 0;
        let mut unknown = Vec::new();
        let mut bad = Vec::new();
        let mut good = 0;
        if check_file == "-" {
            let mut line = String::new();
            loop {
                line.clear();
                stdin().read_line(&mut line).unwrap();
                if line.len() == 0 {
                    break;
                }
                interactive_check(
                    &mut filter,
                    &line,
                    &mut good,
                    &mut bad,
                    &mut unknown,
                    &mut count,
                );
            }
        } else {
            for line in std::fs::read_to_string(check_file).unwrap().lines() {
                println!("'{}'", line);
                interactive_check(
                    &mut filter,
                    line,
                    &mut good,
                    &mut bad,
                    &mut unknown,
                    &mut count,
                );
            }
        }

        println!("");
        println!("checked {} lines:", count);
        println!("{}\tlines good", good);
        println!("{}\tlines bad", bad.len());
        println!("{}\tlines unknown", unknown.len());

        if bad.len() < 50 {
            println!("\nbad words: ");
            for word in bad {
                println!("  {}", word.str());
            }
        }
        if unknown.len() < 50 {
            println!("\nunknown words: ");
            for word in unknown {
                println!("  {}", word.str());
            }
        }
    }

    for path in cli.add_good {
        add_good_file(&mut filter, &path);
    }

    if cli.short_words {
        println!("\nshort words: ");
        let short_words = filter.short_words();
        for (short, good) in &short_words {
            print!("   {}", short);
            if *good {
                println!(" {COLOR_GREEN}(good){RESET}");
            } else {
                println!(" {COLOR_RED}(bad){RESET}");
            }
        }
        println!("{} total", short_words.len());
    }

    // let badwords = std::fs::read_to_string("wordfilter/badwords.txt").unwrap();
    // for line in badwords.lines() {
    //     if line.len() == 0 {
    //         continue;
    //     }
    //     match filter.check(line) {
    //         CheckResult::Good => {
    //             println!("wrongly marked as good: '{}'", line);
    //         }
    //         _ => {}
    //     }
    // }

    println!("done");
    if let Some(output) = cli.output {
        let bytes = filter.save_string().into_bytes();
        println!("filter entries: {}", filter.entry_count());
        println!("filter size: {}kB", bytes.len() / 1000);

        std::fs::write(&output, bytes).unwrap()
    }
}
