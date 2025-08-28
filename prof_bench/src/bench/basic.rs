use crate::{gen_wordlist, other_impls, test_data, PROFANITY_V2};
use profanity::ProfanityFilter;
use std::collections::HashSet;
use test::Bencher;
extern crate test;

mod prof {
    use crate::{gen_wordlist, other_impls, test_data};
    use test::Bencher;
    extern crate test;

    #[bench]
    fn naive_loop(b: &mut Bencher) {
        let list = gen_wordlist();

        b.iter(|| {
            for s in test_data::PROF_SENTENCES {
                assert!(
                    other_impls::sentence_contains_loop(&list, s),
                    "profanity wrongly marked as clean. {}",
                    s
                )
            }
        })
    }
}

mod clean {
    use crate::{gen_wordlist, other_impls, test_data};
    use test::Bencher;
    extern crate test;

    #[bench]
    fn naive_loop(b: &mut Bencher) {
        let list = gen_wordlist();

        b.iter(|| {
            for s in test_data::CLEAN_SENTENCES {
                assert!(
                    !other_impls::sentence_contains_loop(&list, s),
                    "clean wrongly marked as profanity. {}",
                    s
                )
            }
        })
    }
}

#[bench]
fn censor(b: &mut Bencher) {
    let mut list = gen_wordlist();
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

#[bench]
fn v2(b: &mut Bencher) {
    let filter = ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
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
