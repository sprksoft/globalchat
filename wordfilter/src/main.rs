use std::{
    collections::HashSet,
    fs,
    io::{self, stdin, Read, Write},
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

    /// bad data to add to the filter. (this will add all words as bad)
    #[arg(long, short = 'b')]
    add_bad: Vec<String>,

    /// Check the data in the file against the filter (use - for stdin)
    #[arg(long)]
    check: Option<String>,

    /// Answer yes to all questions
    #[arg(short = 'y', long)]
    yes: bool,

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Good,
    Bad,
}

fn ask_mark() -> Mark {
    loop {
        print!("mark (g/b) > ");
        let _ = io::stdout().flush();
        let mut str = String::new();
        io::stdin().read_line(&mut str).unwrap();
        let str = str.trim();
        if str == "g" || str == "good" {
            return Mark::Good;
        } else if str == "b" || str == "bad" {
            return Mark::Bad;
        }
    }
}

fn add_file(filter: &mut WordFilter, path: &str, good: bool, yes: bool) {
    println!("ADDING WORDS FROM: {}", path);
    for line in get_input_data(path)
        .unwrap()
        .lines()
        .map(|l| cleaning::clean_line(l))
        .flatten()
    {
        let str = filter.check(line);
        for (word, tag, norm_word) in str.norm_words() {
            match tag {
                Tag::Unknown => {
                    filter.train_word(word, good);
                }
                Tag::Bad => {
                    if good {
                        println!(
                            "'{}' ({}) ({}) {COLOR_RED}bad{RESET}",
                            line,
                            word,
                            norm_word.root()
                        );
                        if yes || ask_mark() == Mark::Good {
                            filter.train_word(word, true);
                        }
                    }
                }
                Tag::Good => {
                    if !good {
                        println!(
                            "'{}' ({}) ({}) {COLOR_GREEN}good{RESET}",
                            line,
                            word,
                            norm_word.root()
                        );
                        if yes || ask_mark() == Mark::Bad {
                            filter.train_word(word, false);
                        }
                    }
                }
                Tag::Whitespace => {}
            }
        }
    }
}

#[inline]
fn interactive_check(
    filter: &mut WordFilter,
    str: &str,
    good: &mut HashSet<Box<str>>,
    bad: &mut HashSet<(Box<str>, usize)>,
    unknown: &mut HashSet<Box<str>>,
    lines_good: &mut HashSet<Box<str>>,
    linenum: usize,
) {
    let str = filter.check(str);
    let mut wc = 0;
    if str.good() {
        lines_good.insert(str.to_string().into());
    }
    for (word, tag) in str.words() {
        match tag {
            Tag::Good => {
                print!("{COLOR_GREEN}{}{RESET}", word);
                good.insert(word.into());
            }
            Tag::Bad => {
                print!("{COLOR_RED}{}{RESET}", word);
                bad.insert((word.into(), linenum));
            }
            Tag::Unknown => {
                print!("{COLOR_GRAY}{}{RESET}", word);
                unknown.insert(word.into());
            }
            Tag::Whitespace => {
                print!("{}", word);
            }
        }
        wc += 1;
    }

    print!(" ({} ", wc);
    if str.good() {
        println!("{COLOR_GREEN}O{RESET})");
    } else {
        println!("{COLOR_RED}X{RESET})");
    }
}

fn main() {
    let cli = Cli::parse();

    let mut filter = WordFilter::default();
    for filter_path in &cli.filter {
        filter.merge(WordFilter::from_string(
            &std::fs::read_to_string(&filter_path).expect("Could not read filter file"),
        ));
    }

    if let Some(check_file) = cli.check {
        let mut good = HashSet::new();
        let mut bad = HashSet::new();
        let mut unknown = HashSet::new();
        let mut lines_good = HashSet::new();
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
                    &mut lines_good,
                    1,
                );
            }
        } else {
            for (index, line) in std::fs::read_to_string(check_file)
                .unwrap()
                .lines()
                .enumerate()
            {
                println!("'{}'", line);
                interactive_check(
                    &mut filter,
                    line,
                    &mut good,
                    &mut bad,
                    &mut unknown,
                    &mut lines_good,
                    index + 1,
                );
            }
        }

        println!("");
        println!("checked {} lines:", good.len() + bad.len() + unknown.len());
        println!("{}\twords good", good.len());
        println!("{}\twords bad", bad.len());
        println!("{}\twords unknown", unknown.len());
        println!("{}\tlines good", lines_good.len());

        if lines_good.len() < 50 {
            println!("\nlines good: ");
            for line in lines_good.iter() {
                println!("  {}", line);
            }
        }

        if bad.len() < 50 {
            println!("\nbad words:");
            for (word, line) in bad.iter() {
                println!("  {}     \tline: {}", word, line);
            }
        }
        if unknown.len() < 50 {
            println!("\nunknown words: ");
            for word in unknown.iter() {
                println!("  {}", word);
            }
        }
    }

    let mut output = cli.output;

    for path in cli.add_good {
        if output.is_none() && cli.filter.len() == 1 {
            output = Some(cli.filter[0].clone());
        }
        add_file(&mut filter, &path, true, cli.yes);
    }
    for path in cli.add_bad {
        if output.is_none() && cli.filter.len() == 1 {
            output = Some(cli.filter[0].clone());
        }
        add_file(&mut filter, &path, false, cli.yes);
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

    let bytes = filter.save_string().into_bytes();
    println!("filter entries: {}", filter.entry_count());
    println!("filter size: {}kB", bytes.len() / 1000);
    if let Some(output) = output {
        std::fs::write(&output, bytes).unwrap()
    }
}
