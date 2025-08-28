use crate::{test_data, PROFANITY_V2};
use profanity::ProfanityFilter;
use test::Bencher;

extern crate test;

mod basic;

mod prof {
    use crate::{test_data, PROFANITY_V2, WF};
    use profanity::ProfanityFilter;
    use test::Bencher;
    use wordfilter::WordFilter;

    extern crate test;

    #[bench]
    fn v2(b: &mut Bencher) {
        let filter = ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
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
        })
    }

    #[bench]
    fn wf(b: &mut Bencher) {
        let filter = WordFilter::from_string(WF);
        b.iter(|| {
            for s in test_data::PROF_SENTENCES
                .iter()
                .chain(test_data::EXT_PROF_SENTENCES.iter())
            {
                let ts = filter.check(s);
                assert!(
                    !ts.good(),
                    "profanity wrongly marked as clean. '{}'",
                    ts.colored()
                );
            }
        })
    }
}

mod clean {
    use crate::{test_data, PROFANITY_V2, WF};
    use profanity::ProfanityFilter;
    use test::Bencher;
    use wordfilter::WordFilter;

    extern crate test;

    #[bench]
    fn v2(b: &mut Bencher) {
        let filter = ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
        dbg!("{:?}", &filter);

        b.iter(|| {
            for s in test_data::CLEAN_SENTENCES {
                let (tokenized, string) = filter.tokenize(s);
                let result = filter.check(&tokenized);
                assert_eq!(result, None, "clean wrongly marked as profanity. {}", s);
                assert_eq!(string, s, "string was modified by profanity {}", s);
            }
        })
    }

    #[bench]
    fn wf(b: &mut Bencher) {
        let filter = WordFilter::from_string(WF);

        b.iter(|| {
            for s in test_data::CLEAN_SENTENCES {
                let ts = filter.check(s);
                assert!(
                    ts.good(),
                    "profanity wrongly marked as profanity. '{}'",
                    ts.colored()
                );
            }
        })
    }
}

#[bench]
fn v2_modify(b: &mut Bencher) {
    let filter = ProfanityFilter::parse_from_str(PROFANITY_V2).unwrap();
    dbg!("{:?}", &filter);
    b.iter(|| {
        for (s, modify) in test_data::MODIFY_PROF_SENTENCES {
            let (_tokenized, string) = filter.tokenize(s);
            assert_eq!(string, modify, "modification hasn't happened {}", s)
        }
    })
}
