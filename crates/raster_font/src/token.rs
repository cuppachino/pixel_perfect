//! Primitive token and sequence types for defining raster fonts.
//!
//! Two types form the matching vocabulary for raster fonts:
//!
//! - [`Sequence`]: An exact, non-empty string that maps to a single glyph
//!   (`a`, `->`, `:)`). This is the atomic match unit during text parsing.
//!
//! - [`Token`]: A set of one or more [`Sequence`]s that all resolve to the *same*
//!   glyph. Tokens model unions: `$(->|=>)` means `->` and `=>` map to the same glyph.
//!
//! # Relationship to [Layout](crate::layout)
//!
//! An [`OrdTokenLayout`] is an ordered set of [`Token`]s, parsed from a layout string. The position
//! of the token in the layout determines its glyph slot, alongside the [`PackingMode`] of a font.
//!
//! A token contributes exactly one glyph region to a font atlas, regardless of how many
//! sequences it contains.
//!
//! # Parsing
//!
//! Both types implement [`FromStr`] and [`Display`], round-tripping through the
//! layout string syntax. [`Token::parse`] and [`Sequence::new`] are convenience
//! constructors for cases where parse errors are not expected.
//!
//!| Input string   | Parsed as                                                                         | Description                   |
//!| -------------- | --------------------------------------------------------------------------------- | :---------------------------- |
//!| `a`            | `Token([Sequence("a")])`                                                          | one sequence, one char        |
//!| `$(->)`        | `Token([Sequence("->")])`                                                         | one sequence, multiple chars  |
//!| `$(->\|=>)`    | `Token([Sequence("->"), Sequence("=>")])`                                         | either `->` or `=>`           |
//!| `$(\|)`        | `Token([Sequence("\|")])`                                                         | escaped union char `\|`       |
//!| <code>$(\$(\\&#124;\&#124;\\))</code> | `Token([Sequence("$("), Sequence("\|")], Sequence(")")])`  | escaped `$(` or `\|` or `)`   |
//!
//! [`OrdTokenLayout`]: crate::core::OrdTokenLayout
//! [`FromStr`]: std::str::FromStr
//! [`Display`]: std::fmt::Display
//! [`PackingMode`]: crate::meta::PackingMode
#[cfg(feature = "bevy")]
use bevy_reflect::prelude::*;

use crate::{
    collections::HashSet,
    layout::{LayoutParser, Rule},
};
use pest::Parser;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    str::{Bytes, Chars, FromStr},
};

/// A token is one or more [`Sequence`]s that resolve to the same glyph.
///
/// Tokens model the concept of input unions for a glyph slot: if a font
/// has a single sprite for both `->` and `=>`, they are represented as one `Token`
/// containing two sequences. All sequences in a token share exactly one atlas region.
///
/// # Relation to [Layout](crate::layout)
///
/// - `a`: a single-sequence token is written as a bare character.
/// - `$(->)`: a grouped token with one sequence.
/// - `$(->|=>)`: a union token with multiple sequences, separated by `|`.
///
/// ## Example
///
/// ```rust
/// use raster_font::token::{Sequence, Token};
///
/// let one_sequence = Token::parse("a").unwrap();
/// assert_eq!(one_sequence.len(), 1);
/// assert_eq!(one_sequence.first().unwrap().as_str(), "a");
///
/// let two_sequences = Token::parse("$(->|=>)").unwrap();
/// assert_eq!(two_sequences.len(), 2);
/// let mut iter = two_sequences.iter();
/// assert_eq!(iter.next().unwrap().as_str(), "->");
/// assert_eq!(iter.next().unwrap().as_str(), "=>");
/// ```
///
/// # Reserved Syntax
///
/// `$(`, `)`, and `|` are reserved characters that should be escaped with a backslash (`\`) to
/// avoid being parsed as layout syntax.
///
/// For example, the token `$(\$(|\|)` contains the sequences
/// `$(` and `|`, not a union of two tokens.
///
/// ## Example
///
/// ```
/// use raster_font::token::{Token, TokenParsingError};
/// use pest::error::LineColLocation;
///
/// // Don't forget to close all your groups!
/// let line_col = match Token::parse(r#"$(\$(|\|"#).unwrap_err() {
///     TokenParsingError::PestError(e) => e.line_col,
///     _ => unreachable!(),
/// };
/// assert_eq!(line_col, LineColLocation::Pos((1, 9)));
///
/// let t = Token::parse(r#"$(\$(|\|)"#).unwrap();
/// assert_eq!(t.len(), 2);
/// let mut iter = t.iter();
/// assert_eq!(iter.next().unwrap().as_str(), "$(");
/// assert_eq!(iter.next().unwrap().as_str(), "|");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bevy", derive(Reflect), reflect(Debug))]
pub struct Token(Vec<Sequence>);

/// An exact, non-empty input pattern that maps to a glyph.
///
/// A `Sequence` is the **atomic match unit** used during text parsing.
/// It represents an exact string that can be matched in the input stream.
///
/// Properties:
/// - Must be **non-empty**
/// - Matches **exactly** (no regex or partial matching)
/// - Can be multi-character (`->`, `foo`)
/// - Can include normally special syntax as literal text (`$(foo)`)
///
/// Multiple `Sequence`s may map to the same glyph if they grouped together in a `Token` (e.g., `$(->|=>)`).
///
/// Examples:
/// ```text
/// "a"       // single character
/// "->"      // ligature
/// "Samwise" // multi-character sequence
/// "$(lit)"  // literal text that looks like a token
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "bevy", derive(Reflect), reflect(Debug))]
#[serde(transparent)]
pub struct Sequence(String);

impl Sequence {
    /// Create a sequence of characters that identifies a single glyph. This can be a single character,
    /// a multi-character string, or even a string that looks like a token (e.g., `$(foo)`).
    ///
    /// Returns `None` if the input string is empty, since empty sequences are not allowed.
    #[inline]
    pub fn new(s: impl AsRef<str>) -> Option<Self> {
        let s = s.as_ref();
        if s.is_empty() {
            None
        } else {
            Some(Self(s.to_string()))
        }
    }

    /// Create a new sequence without checking if the string is empty.
    pub fn new_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the sequence as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return an iterator of characters in this sequence.
    #[inline]
    pub fn chars(&self) -> Chars<'_> {
        self.0.chars()
    }

    /// Return an iterator of bytes in this sequence.
    #[inline]
    pub fn bytes(&self) -> Bytes<'_> {
        self.0.bytes()
    }

    /// Return the number of characters in this sequence.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    /// Returns `true` if this sequence contains no characters.
    ///
    /// Since empty sequences are not allowed, this should always return `false` for valid `Sequence`s.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Vec<Sequence>> for Token {
    /// Create a token from a union of [`Sequence`]s.
    #[inline]
    fn from(sub_tokens: Vec<Sequence>) -> Self {
        Self(sub_tokens)
    }
}

impl From<Sequence> for Token {
    /// Create a token from a single [`Sequence`].
    #[inline]
    fn from(sub_token: Sequence) -> Self {
        Token(vec![sub_token])
    }
}

impl From<char> for Token {
    #[inline]
    fn from(c: char) -> Self {
        Token::from(Sequence::from(c))
    }
}

impl From<char> for Sequence {
    #[inline]
    fn from(c: char) -> Self {
        Sequence(c.to_string())
    }
}

impl AsRef<[u8]> for Sequence {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsRef<str> for Sequence {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur when trying to convert a `Token` into a single `Sequence`.
#[derive(Debug)]
pub enum IntoSequenceError {
    /// The token contains more than one sequence, so it cannot be converted into a single sequence.
    /// The original token is returned for error reporting.
    ExpectedSingleSequence(Token),
    /// The token is empty and contains no sequences, so it cannot be converted into a sequence.
    EmptyToken,
}

impl Error for IntoSequenceError {}

impl std::fmt::Display for IntoSequenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntoSequenceError::ExpectedSingleSequence(token) => {
                write!(f, "Expected exactly one sequence in token, found {}", token)
            }
            IntoSequenceError::EmptyToken => {
                write!(f, "Cannot convert empty token into a sequence")
            }
        }
    }
}

impl TryFrom<Token> for Sequence {
    type Error = IntoSequenceError;

    /// Try to convert a token into a single sequence.
    ///
    /// ## Errors
    ///
    /// -   If the token is empty (contains no sequences), an [`EmptyToken`] error is returned.
    /// -   If the token contains more than one sequence, an [`ExpectedSingleSequence`] error is
    ///     returned with the original token.
    ///
    /// [`EmptyToken`]: IntoSequenceError::EmptyToken
    /// [`ExpectedSingleSequence`]: IntoSequenceError::ExpectedSingleSequence
    #[inline]
    fn try_from(token: Token) -> Result<Self, Self::Error> {
        if token.len() > 1 {
            Err(IntoSequenceError::ExpectedSingleSequence(token))
        } else {
            token
                .into_iter()
                .next()
                .ok_or(IntoSequenceError::EmptyToken)
        }
    }
}

impl Token {
    /// See also [`FromStr::from_str`] and `str.parse()`.
    ///
    /// [`FromStr::from_str`]: std::str::FromStr::from_str
    #[inline]
    pub fn parse(input: &str) -> Result<Self, TokenParsingError> {
        input.parse()
    }

    /// Returns an iterator of [`Sequence`]s contained in this `Token`.
    ///
    /// It's possible for a sequence to appear multiple times in the same token.
    pub fn iter(&self) -> std::slice::Iter<'_, Sequence> {
        self.0.iter()
    }

    /// Returns the first sequence in this token, if there is one.
    #[inline]
    pub fn first(&self) -> Option<&Sequence> {
        self.0.first()
    }

    /// Returns the number of sequences contained in this token.
    ///
    /// This is not the same as the number of characters in the token, and there is no guarantee
    /// that all sequences are unique.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if this token contains no sequences.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for Token {
    type Item = Sequence;
    type IntoIter = std::vec::IntoIter<Sequence>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Serialize for Token {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.iter().peekable();
        while let Some(sub) = iter.next() {
            match sub.len() {
                0 => write!(f, ""),
                1 => write!(f, "{}", sub.0),
                _ => write!(f, "$({})", sub.0),
            }?;
            if iter.peek().is_some() {
                write!(f, "|")?;
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for Sequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors that can occur when parsing a token from a string.
#[derive(Debug)]
pub enum TokenParsingError {
    PestError(pest::error::Error<Rule>),
    EmptyToken,
    UnexpectedEOI,
}

impl Error for TokenParsingError {}

impl From<pest::error::Error<Rule>> for TokenParsingError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        TokenParsingError::PestError(e)
    }
}

impl std::fmt::Display for TokenParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenParsingError::PestError(e) => write!(f, "{}", e),
            TokenParsingError::EmptyToken => write!(f, "Empty token is not allowed"),
            TokenParsingError::UnexpectedEOI => write!(f, "Unexpected end of input"),
        }
    }
}

impl FromStr for Token {
    type Err = TokenParsingError;

    /// Parse a token from a string. This can be a single character, a multi-character string, or a union of tokens (e.g., `$(a|b|c)`).
    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let mut pairs = match LayoutParser::parse(Rule::single_token, input) {
            Ok(pairs) => pairs,
            Err(e) => {
                return Err(TokenParsingError::PestError(e.renamed_rules(
                    |r| match *r {
                        Rule::EOI => "EOI (Too many chars for a subtoken)".to_string(),
                        _ => format!("{:?}", r),
                    },
                )));
            }
        };

        let Some(pair) = pairs.next() else {
            return Err(TokenParsingError::EmptyToken);
        };

        let mut sub_tokens = Vec::new();

        match pair.as_rule() {
            Rule::any_char => {
                if let Some(sequence) = Sequence::new(pair.as_str()) {
                    sub_tokens.push(sequence);
                }
            }
            Rule::multi_union => {
                let sub_sequences = pair.into_inner();

                let mut string = String::new();

                for sub in sub_sequences {
                    #[cfg(debug_assertions)]
                    debug_assert!(
                        matches!(sub.as_rule(), Rule::sub),
                        "Expected sub rule in multi_union sequence, found {:?}",
                        sub.as_rule()
                    );

                    for seq in sub.into_inner() {
                        // std::debug_assert_matches!(
                        //     seq.as_rule(),
                        //     Rule::any_char | Rule::RESERVED,
                        //     "Expected char or RESERVED rule in multi_union sub-sequence, found {:?}",
                        //     seq.as_rule()
                        // );
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

                    sub_tokens.push(Sequence(std::mem::take(&mut string)));
                }
            }
            Rule::EOI => {
                return Err(TokenParsingError::UnexpectedEOI);
            }
            _ => unreachable!(),
        }

        Ok(Self(sub_tokens))
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TokenVisitor;

        impl<'de> serde::de::Visitor<'de> for TokenVisitor {
            type Value = Token;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a raster font layout token string")
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
                Ok(Token::from(v))
            }
        }

        deserializer.deserialize_any(TokenVisitor)
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn test_simple() {
        println!("{}", Token::parse("$(:))").unwrap());
    }

    #[test]
    fn token_parser() {
        assert_eq!(Token::from('a'), Token::from(Sequence::new_unchecked("a")));
        assert!(Token::from_str(r#"$a"#).is_err());
        assert_eq!(
            Token::from_str(r#"$($a)"#).unwrap(),
            Token::from(Sequence::new_unchecked("$a"))
        );

        assert_eq!(
            Token::from_str("$($bc)").unwrap(),
            Token::from(Sequence::new_unchecked("$bc"))
        );
        assert_eq!(
            Token::from_str("$(abc)").unwrap(),
            Token::from(Sequence::new_unchecked("abc"))
        );
        assert_eq!(
            Token::from_str("$(\\|)").unwrap(),
            Token::from(Sequence::new_unchecked("|"))
        );

        assert!(Token::from_str(r#"\$(abc)"#).is_err());
        assert_eq!(
            Token::from_str(r#"$(\$(abc)"#).unwrap(),
            Token::from(Sequence::new_unchecked("$(abc"))
        );

        assert!(Token::from_str(r#"$(abc\)"#).is_err());
        assert_eq!(
            Token::from_str(r#"$(abc\))"#).unwrap(),
            Token::from(Sequence::new_unchecked("abc)"))
        );
        assert_eq!(
            Token::from_str("$(a|b|c)").unwrap(),
            Token::from(vec![
                Sequence::new_unchecked("a"),
                Sequence::new_unchecked("b"),
                Sequence::new_unchecked("c")
            ])
        );
        assert_eq!(
            Token::from_str(r#"$(a|$bc|\|||foo)"#).unwrap(),
            Token::from(vec![
                Sequence::new_unchecked("a"),
                Sequence::new_unchecked("$bc"),
                Sequence::new_unchecked("|"),
                Sequence::new_unchecked("foo")
            ])
        );
        assert_eq!(
            Token::from_str(r#"$(a|$bc|\||foo|\$($dc\))"#).unwrap(),
            Token::from(vec![
                Sequence::new_unchecked("a"),
                Sequence::new_unchecked("$bc"),
                Sequence::new_unchecked("|"),
                Sequence::new_unchecked("foo"),
                Sequence::new_unchecked("$($dc)")
            ])
        );
    }
}

/// A set of unique sequences in a layout, and the number of glyph regions they would require.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unique<'a> {
    /// A set of all unique sequences in a layout.
    pub sequences: HashSet<&'a Sequence>,
    /// The sum of tokens that contain at least one previously-untracked sequence.
    ///
    /// This is normally used to represent the number of unique glyph regions that should exist
    /// in a font atlas, since tokens that share sequences (unions) can share a glyph region.
    pub num_regions: usize,
}

impl<'a, I> From<I> for Unique<'a>
where
    I: IntoIterator<Item = &'a Token>,
{
    fn from(tokens: I) -> Self {
        let mut sequences = HashSet::new();
        let num_regions = tokens
            .into_iter()
            .map(|token| {
                let mut con = 1;
                for seq in token.iter() {
                    if sequences.contains(seq) {
                        con = 0;
                    }
                    sequences.insert(seq);
                }
                con
            })
            .sum();

        Self {
            sequences,
            num_regions,
        }
    }
}
