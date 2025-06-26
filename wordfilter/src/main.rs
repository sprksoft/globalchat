use std::{
    fs,
    io::{self, stdin, Read},
};

use clap::Parser;
use wordfilter::{CheckResult, WordFilter};

mod cleaning;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// good data to add to the filter interactively.
    #[arg(long, short = 'g')]
    good: Vec<String>,

    /// Check the data in the file against the filter (use - for stdin)
    #[arg(long)]
    check: Option<String>,

    /// Bad data to add to the filter interactively.
    #[arg(long, short = 'b')]
    bad: Vec<String>,

    /// Optional output file of the resulting filter
    /// When the output file ends in .txt write the filter as text and --bad specifies if the bad
    /// or good words should be written
    #[arg(long, short = 'o')]
    output: Option<String>,

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
    println!("{} >", question);
    let mut str = String::new();
    io::stdin().read_line(&mut str).unwrap();
    if str == "y" {
        return true;
    } else {
        return false;
    }
}

fn process_data_file(filter: &mut WordFilter, path: &str, good: bool) {
    print!("PROCESSING: ");
    print!("'{}'", path);
    if good {
        println!(" (good)");
    } else {
        println!(" (bad)");
    }
    for line in get_input_data(path)
        .unwrap()
        .lines()
        .map(|l| cleaning::clean_line(l))
    {
        let Some(line) = line else {
            continue;
        };
        match filter.check(line) {
            CheckResult::Unknown(_) => filter.train(good, line),
            CheckResult::Good if !good => {
                println!("'{}'", line);
                println!("Filter matched as good. but dataset matched as bad");
                println!("bad word >");
                let mut bad_word = String::new();
                io::stdin().read_line(&mut bad_word).unwrap();
                if bad_word.len() != 0 {
                    filter.train(false, &bad_word);
                }
            }
            CheckResult::Bad(word) if good => {
                println!("'{}' ({})", line, word.str());
                println!("Filter matched as bad. but dataset matched as good");
                if ask_yn("mark as good?") {
                    filter.train(true, word.str());
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
    bad: &mut usize,
    unknown: &mut usize,
    count: &mut usize,
) {
    let result = filter.check(&line);
    *count += 1;
    match result {
        CheckResult::Good => {
            println!("good");
            *good += 1;
        }
        CheckResult::Bad(word) => {
            println!("bad: {}", word.str());
            *bad += 1
        }
        CheckResult::Unknown(word) => {
            println!("unknown: {}", word.str());
            *unknown += 1
        }
    }
}

fn main() {
    let cli = Cli::parse();

    let mut filter = WordFilter::default();
    for filter_path in cli.filter {
        filter
            .append_bin(&std::fs::read(filter_path).unwrap())
            .unwrap();
    }

    if let Some(check_file) = cli.check {
        let mut count = 0;
        let mut unknown = 0;
        let mut bad = 0;
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
        println!("{}\tlines bad", bad);
        println!("{}\tlines unknown", unknown);
    }

    for path in cli.good {
        process_data_file(&mut filter, &path, true);
    }
    for path in cli.bad {
        process_data_file(&mut filter, &path, false);
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
        let bytes = if output.ends_with(".txt") {
            filter.save_string().into_bytes()
        } else {
            filter.save_bin().unwrap()
        };
        println!("filter entries: {}", filter.entry_count());
        println!("filter size: {}KB", bytes.len() / 1000);

        std::fs::write(&output, bytes).unwrap()
    }
}
