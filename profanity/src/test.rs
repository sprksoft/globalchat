use super::*;
use std::collections::HashSet;
use test::Bencher;
extern crate test;

const PROF_SENTENCES: [&'static str; 11] = [
    "i am FUCKING green",
    "hellofuckers",
    "niger",
    "zuig mij please",
    "ik eet jou dick op zo",
    "nigga man",
    "k y s",
    "dingely dongs",
    "dingelydongs",
    "k + y + s",
    "https://pornhub.com",
];
const CLEAN_SENTENCES: [&'static str; 9] = [
    "ldev234",
    "ldev2",
    "hallo",
    "ja",
    "hoe gaat die er mee",
    "kom naar mijn huis",
    "whahahahahahhahahah",
    "hallo mannen (en vrouwen) ik ga vandaag een les geven van Pneumatica",
    "Yuww iemand online?",
];

fn gen_list() -> Vec<String> {
    let mut list: Vec<String> = wordlist::LIST
        .lines()
        .map(|w| w.trim_matches('"').to_lowercase())
        .collect();
    list.sort();
    list
}

#[test]
fn matches() {
    assert!(super::matches("ass", "ass "))
}

#[test]
fn test() {
    let filter = ProfanityFilter::from_wordlist(wordlist::LIST);
    println!("ass");
    assert!(filter.contains_profanity("ass"));
    assert!(filter.contains_profanity("you are ass"));
    println!("password");
    assert!(!filter.contains_profanity("password"));
}

#[bench]
fn sentence_contains_censor(b: &mut Bencher) {
    let mut list = gen_list();
    let censor = censor::Custom(HashSet::from_iter(list.drain(..)));
    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(censor.check(s));
        }
        for s in CLEAN_SENTENCES {
            assert!(!censor.check(s));
        }
    })
}

#[bench]
fn sentence_contains(b: &mut Bencher) {
    let list = gen_list();
    let tree = StringTree::from_vec(list.clone());

    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(super::sentence_contains(&tree, s))
        }
        for s in CLEAN_SENTENCES {
            assert!(!super::sentence_contains(&tree, s))
        }
    });
}

#[bench]
fn sentence_contains_naive(b: &mut Bencher) {
    let list = gen_list();

    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(super::sentence_contains_naive(&list, s))
        }
        for s in CLEAN_SENTENCES {
            assert!(!super::sentence_contains_naive(&list, s))
        }
    })
}
