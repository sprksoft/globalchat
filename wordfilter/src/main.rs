use std::{
    collections::HashSet,
    fs,
    io::{self, stdin, Read},
};

use ansii::*;
use clap::Parser;
use wordfilter::{Tag, WordFilter};

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
        .flatten()
    {
        let str = filter.check(line);
        for (word, tag) in str.words() {
            match tag {
                Tag::Unknown => {
                    filter.train_word(word, true);
                }
                Tag::Bad => {
                    println!("'{}' ({}) {COLOR_RED}bad{RESET}", line, word);
                    if ask_yn("mark as good?") {
                        filter.train_word(word, true);
                    }
                }
                Tag::Good => {}
            }
        }
    }
}

#[inline]
fn interactive_check(
    filter: &mut WordFilter,
    str: &str,
    good: &mut HashSet<Box<str>>,
    bad: &mut HashSet<Box<str>>,
    unknown: &mut HashSet<Box<str>>,
) {
    let str = filter.check(str);
    for (word, tag) in str.words() {
        match tag {
            Tag::Good => {
                println!("{COLOR_GREEN}{}{RESET}", word);
                good.insert(word.into());
            }
            Tag::Bad => {
                println!("{COLOR_RED}{}{RESET}", word);
                bad.insert(word.into());
            }
            Tag::Unknown => {
                println!("{COLOR_GRAY}{}{RESET}", word);
                unknown.insert(word.into());
            }
        }
    }
    println!();
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
        let mut good = HashSet::new();
        let mut bad = HashSet::new();
        let mut unknown = HashSet::new();
        if check_file == "-" {
            let mut line = String::new();
            loop {
                line.clear();
                stdin().read_line(&mut line).unwrap();
                if line.len() == 0 {
                    break;
                }
                interactive_check(&mut filter, &line, &mut good, &mut bad, &mut unknown);
            }
        } else {
            for line in std::fs::read_to_string(check_file).unwrap().lines() {
                println!("'{}'", line);
                interactive_check(&mut filter, line, &mut good, &mut bad, &mut unknown);
            }
        }

        println!("");
        println!("checked {} lines:", good.len() + bad.len() + unknown.len());
        println!("{}\tlines good", good.len());
        println!("{}\tlines bad", bad.len());
        println!("{}\tlines unknown", unknown.len());

        if bad.len() < 50 {
            println!("\nbad words: ");
            for word in bad.iter() {
                println!("  {}", word);
            }
        }
        if unknown.len() < 50 {
            println!("\nunknown words: ");
            for word in unknown.iter() {
                println!("  {}", word);
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
