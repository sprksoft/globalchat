use super::*;
use std::collections::HashSet;
use test::Bencher;
extern crate test;

const PROFANITY_V2: &'static str = include_str!("profanity_v2.filter");

const WORD_LIST: &'static str = include_str!("wordlist.txt");

fn gen_list<T: From<String> + std::cmp::Ord>() -> Vec<T> {
    let mut list: Vec<T> = WORD_LIST
        .lines()
        .map(|w| w.trim_matches('"').to_lowercase())
        .filter(|w| w.len() > 0)
        .map(|w| w.into())
        .collect();
    list.sort();
    list
}

#[bench]
fn v2_full(b: &mut Bencher) {
    let filter = crate::ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
    dbg!("{:?}", &filter);

    b.iter(|| {
        for s in test_data::PROF_SENTENCES
            .iter()
            .chain(test_data::EXT_PROF_SENTENCES.iter())
        {
            let (tokenized, _string) = filter.tokenize(s);
            dbg!(&tokenized);
            dbg!(_string);
            let check = filter.check(&tokenized);
            dbg!(&check);
            assert!(check.is_some(), "profanity wrongly marked as clean. {}", s)
        }
        for (s, modify) in test_data::MODIFY_PROF_SENTENCES {
            let (_tokenized, string) = filter.tokenize(s);
            assert_eq!(string, modify, "modification hasn't happened {}", s)
        }
        for s in test_data::CLEAN_SENTENCES {
            let (tokenized, string) = filter.tokenize(s);
            let result = filter.check(&tokenized);
            assert_eq!(result, None, "clean wrongly marked as profanity. {}", s);
            assert_eq!(string, s, "string was modified by profanity {}", s);
        }
    })
}

#[bench]
fn v2_basic(b: &mut Bencher) {
    let filter = crate::ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
    dbg!("{:?}", &filter);

    b.iter(|| {
        for s in test_data::PROF_SENTENCES {
            let tm = filter.tokenize(s).0;
            assert!(
                filter.check(&tm).is_some(),
                "profanity wrongly marked as clean. {}",
                s
            )
        }
        for s in test_data::CLEAN_SENTENCES {
            let tm = filter.tokenize(s).0;
            assert!(
                filter.check(&tm).is_none(),
                "profanity wrongly marked as profanity. {}",
                s
            )
        }
    })
}

#[bench]
fn censor_basic(b: &mut Bencher) {
    let mut list = gen_list();
    let censor = censor::Custom(HashSet::from_iter(list.drain(..)));
    b.iter(|| {
        for s in test_data::PROF_SENTENCES {
            assert!(censor.check(s), "profanity wrongly marked as clean. {}", s)
        }
        for s in test_data::CLEAN_SENTENCES {
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
fn loop_basic(b: &mut Bencher) {
    let list = gen_list();

    b.iter(|| {
        for s in test_data::PROF_SENTENCES {
            assert!(
                super::sentence_contains_loop(&list, s),
                "profanity wrongly marked as clean. {}",
                s
            )
        }
        for s in test_data::CLEAN_SENTENCES {
            assert!(
                !super::sentence_contains_loop(&list, s),
                "clean wrongly marked as profanity. {}",
                s
            )
        }
    })
}
