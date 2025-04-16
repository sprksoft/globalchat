#![cfg_attr(test, feature(test))]
use std::{fmt::Debug, ops::Deref};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod test;
#[cfg(test)]
mod wordlist;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StringTreeRoot {
    inner: StringTree,
}
impl StringTreeRoot {
    pub fn from_vec<T: std::cmp::Ord + Into<Box<str>>>(mut vec: Vec<T>) -> Self {
        vec.sort();
        let mut root_node = StringTree::new_leaf("".into());
        for item in vec.drain(..) {
            let item: Box<str> = item.into();
            root_node.add(item, 0);
        }

        Self { inner: root_node }
    }
    pub fn as_node(&self) -> &StringTree {
        &self.inner
    }
}
impl Deref for StringTreeRoot {
    type Target = StringTree;
    fn deref(&self) -> &Self::Target {
        self.as_node()
    }
}
impl Into<StringTree> for StringTreeRoot {
    fn into(self) -> StringTree {
        self.inner
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StringTree {
    children: Vec<StringTree>,
    str: Box<str>,
}
impl StringTree {
    pub fn from_vec<T: std::cmp::Ord + Into<Box<str>>>(vec: Vec<T>) -> Self {
        StringTreeRoot::from_vec(vec).into()
    }
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

    #[inline]
    fn common_str<'a>(str1: &'a str, str2: &str) -> &'a str {
        for (i, (byte1, byte2)) in str1.bytes().zip(str2.bytes()).enumerate() {
            if byte1 != byte2 {
                let mut i = i;
                while !str1.is_char_boundary(i) {
                    i -= 1;
                }
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
                if new_item == self.str {
                    return None;
                }
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

    pub fn contains<F: Fn(&str, usize) -> bool>(&self, starts_with: &F) -> (bool, usize) {
        self._contains(0, starts_with)
    }

    fn _contains<F: Fn(&str, usize) -> bool>(
        &self,
        start_index: usize,
        starts_with: &F,
    ) -> (bool, usize) {
        if self.leaf() {
            (starts_with(&self.str.as_ref(), start_index), start_index)
        } else {
            if starts_with(self.str.as_ref(), start_index) {
                for child in self.children.iter() {
                    let result = child
                        ._contains(start_index + (&self.str[start_index..]).len(), starts_with);
                    if result.0 {
                        return result;
                    }
                }
                (false, start_index)
            } else {
                (false, start_index)
            }
        }
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
