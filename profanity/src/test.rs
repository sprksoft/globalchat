use super::*;
use std::collections::HashSet;
use test::Bencher;
extern crate test;

const PROF_SENTENCES: [&'static str; 41] = [
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
    "k.y.s",
    "https://pornhub.com",
    "sexting",

    "zwarte mensen mafia",

    "ik lik je tieten af van meisjes met dikke teieten",

    "oke iedereen is dom en gay",

    "wie niet antwoord is dom en gay",

    //"iemand die een muur te komen bouwen met mij?",

    //"kijk hij is zo gy",

    "Ik zweer op de kop van sinterklaas",

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

    "https://pornhub.com",

    "niger",

    "n!iiiiiger",

    "HOEMOE PAKISTAN",

    "YOO IK BEN EEN NIGG2R",

    //"Zeg eerst uw snap dan",

    "nigerdigger",

    "fat ass",

    "kurwa",

    "Machine I will cut you down, break you appart, splay the gore of your profane form across the stars. I will grind you down until the very sparks cry for mercy. My hands shall relish ending you here and now.",

    "nueken",

    //"lesbîeb",

    //"gây",

    "neuk",

    "piemel",
    ];

// prof sentences not caught by censor to see performance difference
const EXT_PROF_SENTENCES: [&'static str; 7] = [
    "niger",
    "n!iiiiiiger",
    "niggggggggggger",
    "njggggr",
    "69696293",
    "nigerdigger",
    "nîger",
];

const MODIFY_PROF_SENTENCES: [(&'static str, &'static str); 7] = [
    ("so hot 💦💦💦", "so hot "),
    ("🍑🍑", ""),
    ("]ð 3]", " 3"),
    ("x 3]", "x 3"),
    ("Ÿð", ""),
    ("🎄🎄🎄🎄🎄", ""),
    ("fun@gmail.com", "fungmail.com"),
];

const CLEAN_SENTENCES: [&'static str; 14] = [
    "ldev234",
    ":smppgc:",
    "so hot",
    "ldev2",
    "hallo",
    "ja",
    "hoe gaat die er mee",
    "kom naar mijn huis",
    "whahahahahahhahahah",
    "waaaa",
    "hallo mannen (en vrouwen) ik ga vandaag een les geven van Pneumatica",
    "Yuww iemand online?",
    "Hallo hoe gaat die",
    "ik schreef da met 2 k's",
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

#[bench]
fn sentence_contains_v2(b: &mut Bencher) {
    let filter = ProfanityFilter2::from_str(wordlist::PROFANITY_V2).unwrap();
    println!("{:?}", filter);

    b.iter(|| {
        for s in PROF_SENTENCES.iter().chain(EXT_PROF_SENTENCES.iter()) {
            let (tokenized, string) = filter.tokenize(s);
            assert!(
                filter.find_matching(tokenized).is_some(),
                "profanity wrongly marked as clean. {}",
                s
            )
        }
        for (s, modify) in MODIFY_PROF_SENTENCES {
            let (tokenized, string) = filter.tokenize(s);
            assert_eq!(string, modify, "modification hasn't happened {}", s)
        }
        for s in CLEAN_SENTENCES {
            let (tokenized, string) = filter.tokenize(s);
            let result = filter.find_matching(tokenized);
            assert_eq!(result, None, "clean wrongly marked as profanity. {}", s);
            assert_eq!(string, s, "string was modified by profanity {}", s);
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
