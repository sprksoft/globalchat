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

fn gen_list() -> Vec<String> {
    let mut list: Vec<String> = wordlist::LIST.lines().map(|w| w.to_lowercase()).collect();
    list.sort();
    list
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
        &|check: &str, index| {
            let value = $word[index..].starts_with(&check[index..]);
            //println!("{} {} {} = {}", $word, check, index, value);
            value
        }
    };
}

#[bench]
fn bench_build_tree(b: &mut Bencher) {
    let list = gen_list();
    b.iter(|| StringTree::from_vec(list.clone()))
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
            assert!(tree.contains(starts_with!(word)).0);
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
        assert!(!tree.contains(starts_with!("bo")).0);
        assert!(!tree.contains(starts_with!("")).0);
        for word in list.iter() {
            assert!(tree.contains(starts_with!(word)).0);
        }
    })
}
