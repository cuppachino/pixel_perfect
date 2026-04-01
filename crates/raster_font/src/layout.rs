//! Representation and parsing of ordered font token layouts.
//!
//! A layout string is a series of *[`Token`] expressions*, parsed left to right. Whitespace is
//! preserved and significant. Token order determines how glyphs are associated with atlas regions.
//! Each token contributes exactly one glyph slot in an atlas, but may contain one or more valid
//! input sequences mapped to that [`AtlasIndex`].
//!
//! Layouts and tokens may appear similar in string form, but they represent
//! different concepts:
//!
//! - A layout is an ordered *list of tokens*.
//! - A token is a *set of sequences* that resolve to the same glyph.
//!
//! Attempting to parse a layout string as a [`Token`] will fail, since a token
//! cannot contain multiple tokens.
//!
//! # Examples
//!
//! Parsing a simple layout string:
//!
//! ```
//! use raster_font::core::OrdTokenLayout;
//!
//! let layout = OrdTokenLayout::parse("abc");
//! let unique = layout.unique();
//! assert_eq!(unique.num_regions, 3);
//! assert_eq!(unique.sequences.len(), 3);
//!
//! let mut token_iter = layout.iter_tokens();
//! assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from('a'))));
//! assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from('b'))));
//! assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from('c'))));
//! assert_eq!(token_iter.next(), None);
//! ```
//!
//! Union tokens allow alternate input patterns for a single glyph slot:
//!
//! ```
//! use raster_font::core::OrdTokenLayout;
//!
//! // One glyph slot that accepts either "Hello" or "hello", followed by a "!" slot.
//! let layout = OrdTokenLayout::parse("$(Hello|hello)!");
//! let unique = layout.unique();
//! assert_eq!(unique.num_regions, 1);
//! assert_eq!(unique.sequences.len(), 2);
//!
//! let mut token_iter = layout.iter_tokens();
//! assert_eq!(
//!     token_iter.next(),
//!     Some(&Token::from(vec![
//!         Sequence::from("Hello"),
//!         Sequence::from("hello")
//!     ]))
//! );
//! assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from('!'))));
//! assert_eq!(token_iter.next(), None);
//! ```
//!
//! # Serialization
//!
//! [`OrdTokenLayout`] implements [`Serialize`] and [`Deserialize`], round-tripping through
//! its layout string representation. It also implements [`Display`] and [`FromStr`] for
//! convenient string conversion.
//!
//! [`AtlasIndex`]: crate::core::AtlasIndex
//! [`Token`]: crate::token::Token
//! [`Sequence`]: crate::token::Sequence
//! [`Display`]: std::fmt::Display
//! [`FromStr`]: std::str::FromStr
//! [`Serialize`]: serde::Serialize
//! [`Deserialize`]: serde::Deserialize

use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};

use std::str::FromStr;

use crate::{
    core::Unique,
    token::{Sequence, Token, TokenParsingError},
};

mod _parser {
    use super::*;

    #[derive(Parser)]
    #[grammar = "layout.pest"]
    pub(crate) struct LayoutParser;
}
pub(crate) use _parser::{LayoutParser, Rule};

/// An ordered sequence of font tokens, commonly parsed from a layout string.
///
/// - Each [`Token`] represents **one glyph slot**.
/// - Each [`Sequence`] inside a `Token` is an **accepted input pattern** for that glyph.
///
/// ## Example
///
/// ```
/// use raster_font::core::{OrdTokenLayout, Token, Sequence};
///
/// let str_layout = r#"a$(\$(bc|de))"#;
/// let ord_layout = str_layout.parse::<OrdTokenLayout>().unwrap();
///
/// let unique = ord_layout.unique();
/// assert_eq!(unique.num_regions, 3);
/// assert_eq!(unique.sequences.len(), 4);
///
/// let mut token_iter = ord_layout.iter_tokens();
/// assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from('a'))));
/// assert_eq!(token_iter.next(), Some(&Token::from(vec![Sequence::new_unchecked("$(bc"), Sequence::new_unchecked("de")])));
/// assert_eq!(token_iter.next(), Some(&Token::from(Sequence::from(')'))));
/// assert_eq!(token_iter.next(), None);
///
/// let mut seq_iter = ord_layout.iter_sequences();
/// assert_eq!(seq_iter.next(), Some(&Sequence::from('a')));
/// assert_eq!(seq_iter.next(), Some(&Sequence::new_unchecked("$(bc")));
/// assert_eq!(seq_iter.next(), Some(&Sequence::new_unchecked("de")));
/// assert_eq!(seq_iter.next(), Some(&Sequence::from(')')));
/// assert_eq!(seq_iter.next(), None);
/// ```
///
/// [`Token`]: crate::token::Token
/// [`Sequence`]: crate::token::Sequence
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OrdTokenLayout(Vec<Token>);

impl AsRef<[Token]> for OrdTokenLayout {
    #[inline]
    fn as_ref(&self) -> &[Token] {
        &self.0
    }
}

impl OrdTokenLayout {
    /// Returns an iterator of all tokens in `self`, in order. Each token represents a single
    /// glyph slot, and may contain one or more accepted input patterns (sequences) for that glyph.
    ///
    /// For a flattened iterator over all sequences in the layout, see [`iter_sequences`].
    ///
    /// [`iter_sequences`]: Self::iter_sequences
    #[inline]
    pub fn iter_tokens(&self) -> std::slice::Iter<'_, Token> {
        self.0.iter()
    }

    /// Returns an iterator of all sequences in `self`, in order.
    ///
    /// A [`Sequence`] may appear multiple times during iteration if it is a member of more than
    /// one [`Token`]. For a non-flattened iterator over *tokens*, see [`iter_tokens`].
    /// To obtain a set of unique sequences in the layout, see [`unique`].
    ///
    /// [`iter_tokens`]: Self::iter_tokens
    /// [`unique`]: Self::unique
    #[inline]
    pub fn iter_sequences(&self) -> impl Iterator<Item = &Sequence> {
        self.iter_tokens().flat_map(Token::iter)
    }

    /// Returns statistics about the number of glyph regions and unique sequences in this layout.
    #[doc(alias = "glyph count")]
    #[doc(alias = "region count")]
    #[doc(alias = "unique sequence")]
    #[inline]
    pub fn unique<'a>(&'a self) -> Unique<'a> {
        self.iter_tokens().into()
    }

    /// Push a new token into the layout, and return the modified layout for chaining.
    #[inline]
    pub fn with(mut self, token: impl Into<Token>) -> Self {
        self.0.push(token.into());
        self
    }

    /// Push a new token into the layout.
    #[inline]
    pub fn push(&mut self, token: impl Into<Token>) {
        self.0.push(token.into());
    }

    /// Associated convenience function to parse a token layout without calling `unwrap` on the
    /// `Result` of [`Self::from_str`].
    ///
    /// # Panics
    ///
    /// Panics if the token is empty. Use [`FromStr::from_str`] or `str::parse`
    /// to handle parsing errors gracefully.
    ///
    /// [`FromStr::from_str`]: std::str::FromStr::from_str
    pub fn parse(input: impl AsRef<str>) -> Self {
        input.as_ref().parse().unwrap()
    }
}

impl FromStr for OrdTokenLayout {
    type Err = TokenParsingError;

    #[inline]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let pairs = LayoutParser::parse(Rule::tokens, input)?;

        let mut tokens = Vec::new();
        let mut sub_tokens = Vec::new();
        let mut string = String::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::any_char => {
                    let s = pair.as_str();
                    if s.is_empty() {
                        return Err(TokenParsingError::EmptyToken);
                    } else {
                        tokens.push(Token::from(Sequence::new_unchecked(s)));
                    }
                }
                Rule::multi_union => {
                    let sub_sequences = pair.into_inner();

                    for sub in sub_sequences {
                        #[cfg(debug_assertions)]
                        debug_assert!(
                            matches!(sub.as_rule(), Rule::sub),
                            "Expected sub rule in multi_union sequence, found {:?}",
                            sub.as_rule()
                        );

                        for seq in sub.into_inner() {
                            #[cfg(debug_assertions)]
                            debug_assert!(
                                matches!(seq.as_rule(), Rule::any_char | Rule::RESERVED),
                                "Expected any_char or RESERVED rule in multi_union::sub::seq, found {:?}",
                                seq.as_rule()
                            );

                            let s = seq.as_str();
                            string.push_str(s);
                        }

                        if string.is_empty() {
                            return Err(TokenParsingError::EmptyToken);
                        }

                        sub_tokens.push(Sequence::new_unchecked(std::mem::take(&mut string)));
                    }

                    tokens.push(Token::from(std::mem::take(&mut sub_tokens)));
                }
                Rule::EOI => {
                    break;
                }
                unknown => {
                    unreachable!("Unexpected rule while parsing layout tokens: {:?}", unknown)
                }
            }
        }

        #[cfg(debug_assertions)]
        debug_assert!(
            sub_tokens.is_empty(),
            "Sequences should be cleared after processing a multi_union"
        );

        Ok(Self(tokens))
    }
}

impl<'de> Deserialize<'de> for OrdTokenLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RasterFontLayoutVisitor;

        impl<'de> serde::de::Visitor<'de> for RasterFontLayoutVisitor {
            type Value = OrdTokenLayout;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a raster font layout string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(serde::de::Error::custom)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(serde::de::Error::custom)
            }

            fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrdTokenLayout(vec![Token::from(Sequence::from(v))]))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let s = std::str::from_utf8(v).map_err(serde::de::Error::custom)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_any(RasterFontLayoutVisitor)
    }
}

impl Serialize for OrdTokenLayout {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = String::new();
        for token in &self.0 {
            s.push_str(&token.to_string());
        }
        serializer.serialize_str(&s)
    }
}

impl std::fmt::Display for OrdTokenLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.0 {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_parser() {
        let text = r#"a$\\($(bc)d$(ef)$($g)$(h|(H) |$(x|y)$(|||)$(||a|)$(\||a|)"#;
        let layout = OrdTokenLayout::parse(text);

        let expected = OrdTokenLayout(vec![
            Token::from(Sequence::from('a')),
            Token::from(Sequence::from('$')),
            Token::from(Sequence::from('\\')),
            Token::from(Sequence::from('(')),
            Token::from(Sequence::new_unchecked("bc")),
            Token::from(Sequence::from('d')),
            Token::from(Sequence::new_unchecked("ef")),
            Token::from(Sequence::new_unchecked("$g")),
            Token::from(vec![Sequence::from('h'), Sequence::new_unchecked("(H")]),
            Token::from(Sequence::from(' ')),
            Token::from(Sequence::from('|')),
            Token::from(vec![Sequence::from('x'), Sequence::from('y')]),
            Token::from(Sequence::from('a')),
            Token::from(vec![Sequence::from('|'), Sequence::from('a')]),
        ]);

        for (i, token) in layout.iter_tokens().enumerate() {
            println!("Token {}: {}", i, token);
            assert_eq!(token, &expected.0[i]);
        }
    }

    fn remove_spaces(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    #[test]
    fn can_escape() {
        let text = r#"$( a \4 \$(b\) | $b )c\d\$($\()$(x)"#;
        let text = remove_spaces(text);
        let tokens = OrdTokenLayout::from_str(&text).unwrap();

        let expected = OrdTokenLayout(vec![
            Token::from(vec![
                Sequence::new_unchecked("a4$(b)"),
                Sequence::new_unchecked("$b"),
            ]),
            Token::from(Sequence::new_unchecked("c")),
            Token::from(Sequence::new_unchecked("d")),
            Token::from(Sequence::new_unchecked("$")),
            Token::from(Sequence::new_unchecked("(")),
            Token::from(Sequence::new_unchecked("$")),
            Token::from(Sequence::new_unchecked("(")),
            Token::from(Sequence::new_unchecked(")")),
            Token::from(Sequence::new_unchecked("x")),
        ]);
        assert_eq!(tokens, expected);
    }

    #[test]
    fn with_spaces() -> Result<(), TokenParsingError> {
        let text = r#"$( a \4 \$(b\) | $b )"#;

        let tokens = OrdTokenLayout::from_str(text)?;
        let expected = OrdTokenLayout(vec![Token::from(vec![
            Sequence::new_unchecked(" a 4 $(b) "),
            Sequence::new_unchecked(" $b "),
        ])]);
        assert_eq!(tokens, expected);

        Ok(())
    }
}
