use super::*;
extern crate test;
use test::Bencher;

#[test]
fn common_str() {
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
    let tree = StringTree::from_iter(wordlist.lines());
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

#[test]
fn contains() {
    let tree = StringTree::from_iter(wordlist::LIST.lines());
    assert!(tree.contains("able"));
    assert!(tree.contains("your"));
}
#[test]
fn naive_contains() {
    let list = wordlist::LIST.lines().collect();
    assert!(super::naive_contains("able", &list));
    assert!(super::naive_contains("your", &list));
}

#[bench]
fn build_tree(b: &mut Bencher) {
    b.iter(|| StringTree::from_iter(wordlist::LIST.lines()))
}

#[bench]
fn bench_naive_contains(b: &mut Bencher) {
    let list: Vec<&'static str> = wordlist::LIST.lines().collect();
    b.iter(|| super::naive_contains("word", &list))
}

#[bench]
fn bench_contains(b: &mut Bencher) {
    let tree = StringTree::from_iter(wordlist::LIST.lines());
    b.iter(|| tree.contains("word"))
}
