use super::*;
extern crate test;
use test::Bencher;

#[test]
fn common_str_test() {
    assert_eq!(StringTree::common_str("apple", "able"), "a");
    assert_eq!(StringTree::common_str("able", "apple"), "a");
    assert_eq!(StringTree::common_str("box", "boy"), "bo");
    assert_eq!(StringTree::common_str("boy", "box"), "bo");
    assert_eq!(StringTree::common_str("bo", "box"), "bo");
    assert_eq!(StringTree::common_str("cow", "through"), "");
}

#[test]
fn build_test() {
    let wordlist = "about\nable\nability\nboy\nbox\ncall\napple\nabove";
    let list = wordlist.lines().collect();
    let tree = StringTree::from_vec(list);
    let manual = string_tree!(
        "":{
            "a":{
                "apple",
                "ab":{
                    "able",
                    "ability",
                    "abo":{
                        "about",
                        "above"
                    }
                }
            },
            "bo":{
                "boy",
                "box"
            },
            "call"
        }
    );

    assert_eq!(tree, manual);
}

macro_rules! starts_with {
    ($word:expr) => {
        &|check, index| {
            let value = $word[index..].starts_with(&check[index..]);
            //println!("{} {} {} = {}", $word, check, index, value);
            value
        }
    };
}

const PROF_SENTENCES: [&'static str; 5] = [
    "i am fucking green",
    "niger",
    "zuig mij please",
    "ik eet jou dick op zo",
    "nigga man",
];
const CLEAN_SENTENCES: [&'static str; 6] = [
    "hallo",
    "ja",
    "hoe gaat die er mee",
    "kom naar mijn huis",
    "whahahahahahhahahah",
    "hallo mannen (en vrouwen) ik ga vandaag een les geven van Pneumatica",
];

fn gen_list() -> Vec<String> {
    let mut list: Vec<String> = wordlist::LIST_SMALL
        .lines()
        .map(|w| w.to_lowercase())
        .collect();
    list.sort();
    list
}

#[bench]
fn bench_build_tree(b: &mut Bencher) {
    let list = gen_list();
    b.iter(|| StringTree::from_vec(list.clone()))
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
    })
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

#[bench]
fn contains_small_naive(b: &mut Bencher) {
    let list = gen_list();
    b.iter(|| {
        for word in list.iter() {
            assert!(super::contains_naive(word, &list));
        }
    })
}

#[bench]
fn contains_small(b: &mut Bencher) {
    let list = gen_list();
    let tree = StringTree::from_vec(list.clone());
    b.iter(|| {
        for word in list.iter() {
            assert!(tree.contains(0, starts_with!(word)).0);
        }
    })
}

#[bench]
fn contains_naive(b: &mut Bencher) {
    let list = gen_list();
    b.iter(|| {
        assert!(!super::contains_naive("bo", &list));
        assert!(!super::contains_naive("", &list));
        for word in list.iter() {
            assert!(super::contains_naive(word, &list));
        }
    })
}

#[bench]
fn contains(b: &mut Bencher) {
    let list = gen_list();
    let tree = StringTree::from_vec(list.clone());
    b.iter(|| {
        assert!(!tree.contains(0, starts_with!("bo")).0);
        assert!(!tree.contains(0, starts_with!("")).0);
        for word in list.iter() {
            assert!(tree.contains(0, starts_with!(word)).0);
        }
    })
}
