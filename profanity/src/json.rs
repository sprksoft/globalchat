use std::{marker::PhantomData, num::NonZeroU8};

use serde::de::Visitor;

use crate::{Token, TokenGroup};

pub(crate) struct TokenVisitor;
impl<'de> Visitor<'de> for TokenVisitor {
    type Value = Token;
    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("a profanity token")
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        NonZeroU8::new(v)
            .map(|v| Token::from_u8(v))
            .flatten()
            .ok_or(E::custom("number is an invalid token"))
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v <= u32::from(u8::MAX) {
            self.visit_u8(v as u8)
        } else {
            Err(E::custom("number is an invalid token"))
        }
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v <= i32::from(u8::MAX) && v >= i32::from(u8::MIN) {
            self.visit_u8(v as u8)
        } else {
            Err(E::custom("number is an invalid token"))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v <= u64::from(u8::MAX) {
            self.visit_u8(v as u8)
        } else {
            Err(E::custom("number is an invalid token"))
        }
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v <= i64::from(u8::MAX) && v >= i64::from(u8::MIN) {
            self.visit_u8(v as u8)
        } else {
            Err(E::custom("number is an invalid token"))
        }
    }
}

pub(crate) struct TokenGroupVisitor;

impl<'de> Visitor<'de> for TokenGroupVisitor {
    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("a profanity token group")
    }
    type Value = TokenGroup;
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut tokens = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(token) = seq.next_element()? {
            tokens.push(token);
        }

        Ok(if tokens.len() == 1 {
            TokenGroup::from_single(tokens[0])
        } else {
            TokenGroup::from(tokens)
        })
    }
}

pub(crate) struct FlagsVisitor<F: crate::rules::Flags>(pub PhantomData<F>);

impl<'de, F: crate::rules::Flags> Visitor<'de> for FlagsVisitor<F> {
    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("flag sequence")
    }
    type Value = F;
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut flags = F::none();

        while let Some(el) = seq.next_element()? {
            flags.set_from_str(el);
        }

        Ok(flags)
    }
}
