#![feature(test)]
use std::{fmt::Debug, rc::Rc};

#[cfg(test)]
mod test;
mod wordlist;

pub struct StringTree {
    children: Vec<StringTree>,
    str: Box<str>,
}
impl StringTree {
    pub fn new_node(str: Box<str>, children: Vec<StringTree>) -> Self {
        Self { children, str }
    }
    pub fn new_leaf(str: Box<str>) -> Self {
        Self {
            children: vec![],
            str,
        }
    }
    pub fn leaf(&self) -> bool {
        self.children.len() == 0
    }
}
impl PartialEq for StringTree {
    fn eq(&self, other: &Self) -> bool {
        if self.str != other.str {
            return false;
        }
        for child in self.children.iter() {
            if !other.children.contains(&child) {
                return false;
            }
        }

        true
    }
}
impl Debug for StringTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"")?;
        f.write_str(&self.str)?;
        f.write_str("\"")?;
        if !self.leaf() {
            f.write_str(":{ ")?;
            let mut iter = self.children.iter();
            if let Some(first_item) = iter.next() {
                first_item.fmt(f)?;
            }
            for child in iter {
                f.write_str(", ")?;
                child.fmt(f)?;
            }

            f.write_str(" }")?;
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! string_tree {
    ($leaf:literal) => {
        StringTree::new_leaf($leaf.into())
    };

    ($str:literal:{$($node:literal$(:$children:tt)?),*}) => {
        StringTree::new_node($str.into(), vec![$(string_tree!($node$(:$children)?)),*])
    };
}

impl StringTree {
    pub fn from_iter(iter: impl Iterator<Item = impl Into<Box<str>>>) -> Self {
        let mut root_node = Self::new_leaf("".into());
        for item in iter {
            let into = item.into();
            root_node.add(into, 0);
        }

        root_node
    }

    #[inline]
    fn common_str<'a>(str1: &'a str, str2: &str) -> &'a str {
        for (i, (byte1, byte2)) in str1.bytes().zip(str2.bytes()).enumerate() {
            if byte1 != byte2 {
                return &str1[..i];
            }
        }
        str1
    }

    #[inline]
    fn split(&mut self, mut common_str: Box<str>) {
        std::mem::swap(&mut common_str, &mut self.str);
        if common_str.len() == 0 {
            return;
        }
        let mut new_children = Vec::with_capacity(2);
        new_children.push(Self::new_leaf(common_str));
        std::mem::swap(&mut self.children, &mut new_children);
        self.children[0].children = new_children;
    }

    pub fn add(&mut self, mut new_item: Box<str>, min_common: usize) -> Option<Box<str>> {
        let common = Self::common_str(self.str.as_ref(), new_item.as_ref());
        if common.len() >= min_common {
            if self.leaf() {
                self.split(common.into());
                self.children.push(Self::new_leaf(new_item));
                None
            } else {
                for child in self.children.iter_mut() {
                    new_item = child.add(new_item, common.len() + 1)?;
                }
                if self.str.len() == common.len() {
                    self.children.push(Self::new_leaf(new_item));
                    None
                } else {
                    self.split(common.into());
                    self.children.push(Self::new_leaf(new_item));
                    None
                }
            }
        } else {
            Some(new_item)
        }
    }

    fn contains(&self, word: &str) -> bool {
        if self.leaf() {
            self.str.as_ref() == word
        } else {
            if word.starts_with(self.str.as_ref()) {
                for child in self.children.iter() {
                    if child.contains(word) {
                        return true;
                    }
                }
                false
            } else {
                false
            }
        }
    }
}
pub enum CmpResult {
    Next,
    Equal,
    NotEqual,
}
impl From<std::cmp::Ordering> for CmpResult {
    fn from(value: std::cmp::Ordering) -> Self {
        match value {
            std::cmp::Ordering::Equal => Self::Equal,
            _ => Self::NotEqual,
        }
    }
}

fn naive_contains(target: &str, tree: &Vec<&'static str>) -> bool {
    for item in tree.iter() {
        if *item == target {
            return true;
        }
    }
    false
}
