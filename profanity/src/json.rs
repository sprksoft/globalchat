use std::marker::PhantomData;

use serde::de::Visitor;

use crate::TokenGroup;

pub(crate) struct TokenGroupVisitor;

impl<'de> Visitor<'de> for TokenGroupVisitor {
    fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("a TokenGroup")
    }
    type Value = TokenGroup;
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut tokens = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(el) = seq.next_element()? {
            tokens.push(el);
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
