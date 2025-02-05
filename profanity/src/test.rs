use super::*;
use std::collections::HashSet;
use test::Bencher;
extern crate test;

const PROF_SENTENCES: [&'static str; 14] = [
    "i am FUCKING green",
    "hellofuckers",
    "zuig mij please",
    "ik eet jou dick op zo",
    "nigga man",
    "k y s",
    "dingely dongs",
    "dingelydongs",
    "+k + y + s",
    "n-1gg4",
    "nigga",
    "🍑🍑",
    "https://pornhub.com",
    "so hot 💦💦💦",
];

// prof sentences not caught by censor to see performance difference
const EXT_PROF_SENTENCES: [&'static str; 4] = [
    "niger",
    "n!iiiiiiger",
    "niggggggggggger",
    "njggggr",
    //"nigerdigger", //TODO: need whitelist for this to not be confused with nigeria
];

const CLEAN_SENTENCES: [&'static str; 19] = [
    "ldev234",
    "so hot",
    "ldev2",
    "hallo",
    "ja",
    "hoe gaat die er mee",
    "kom naar mijn huis",
    "whahahahahahhahahah",
    "waaaa",
    "x 3]",
    "]ð 3]",
    "Ÿð",
    "hallo mannen (en vrouwen) ik ga vandaag een les geven van Pneumatica",
    "Yuww iemand online?",
    "Hallo hoe gaat die 😊",
    "🎄🎄🎄🎄🎄",
    "ik schreef da met 2 k's",
    "fun@gmail.com",
    "69696293",
];

// clean sentences not caught by censor to see performance difference
const EXT_CLEAN_SENTENCES: [&'static str; 2] = ["nigeria", "password"];

fn gen_list<T: From<String> + std::cmp::Ord>() -> Vec<T> {
    let mut list: Vec<T> = wordlist::LIST
        .lines()
        .map(|w| w.trim_matches('"').to_lowercase())
        .filter(|w| w.len() > 0)
        .map(|w| w.into())
        .collect();
    list.sort();
    list
}

#[test]
fn matches() {
    assert!(super::matches("ass", "ass "));
    assert!(!super::matches("password", "ass "));
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
fn sentence_contains_v2(b: &mut Bencher) {
    let list = gen_list();
    let mut filter = ProfanityFilter2::empty();
    for item in list {
        filter.insert_rule(ProfRule::from_str(str))
    }

    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(
                super::sentence_contains_loop(&list, s),
                "profanity wrongly marked as clean. {}",
                s
            )
        }
        for s in CLEAN_SENTENCES {
            assert!(
                !super::sentence_contains_loop(&list, s),
                "clean wrongly marked as profanity. {}",
                s
            )
        }
    })
}

#[bench]
fn sentence_contains_censor(b: &mut Bencher) {
    let mut list = gen_list();
    let censor = censor::Custom(HashSet::from_iter(list.drain(..)));
    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(censor.check(s), "profanity wrongly marked as clean. {}", s)
        }
        for s in CLEAN_SENTENCES {
            assert!(
                !censor.check(s),
                "profanity wrongly marked as profanity. {}",
                s
            )
        }
    })
}

/* #[bench]
fn sentence_contains_stringtree(b: &mut Bencher) {
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
} */

#[bench]
fn sentence_contains_loop(b: &mut Bencher) {
    let list = gen_list();

    b.iter(|| {
        for s in PROF_SENTENCES {
            assert!(
                super::sentence_contains_loop(&list, s),
                "profanity wrongly marked as clean. {}",
                s
            )
        }
        for s in CLEAN_SENTENCES {
            assert!(
                !super::sentence_contains_loop(&list, s),
                "clean wrongly marked as profanity. {}",
                s
            )
        }
    })
}

#[test]
fn sentence_contains_loop_test() {
    let list = gen_list();
    for s in PROF_SENTENCES.iter().chain(EXT_PROF_SENTENCES.iter()) {
        assert!(
            super::sentence_contains_loop(&list, s),
            "profanity wrongly marked as clean. {}",
            s
        )
    }
    for s in CLEAN_SENTENCES.iter().chain(EXT_CLEAN_SENTENCES.iter()) {
        assert!(
            !super::sentence_contains_loop(&list, s),
            "clean wrongly marked as profanity. {}",
            s
        )
    }
}
