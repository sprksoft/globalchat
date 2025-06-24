use std::io::stdin;

use wordfilter::WordFilter;

fn main() {
    let mut filter = WordFilter::default();

    let data = std::fs::read_to_string("wordfilter/goodwords.txt").unwrap();
    filter.train(true, &data);

    let mut input = String::new();
    loop {
        stdin().read_line(&mut input).unwrap();
        println!("{}", filter.check(&input));
        input.clear();
    }
}
