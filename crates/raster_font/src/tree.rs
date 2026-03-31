//! Efficient multi-character sequence resolution via Aho-Corasick matching.
//!
//! [`LigatureTree`] wraps an [Aho-Corasick] automaton to resolve multi-character
//! sequences to their associated values in a single left-to-right pass over an input string.
//! The name "tree" reflects its role in the font pipeline rather than its internal structure.
//!
//! # Match Semantics
//!
//! Matches are **leftmost-longest**: at any position in the input, the longest
//! registered sequence wins, and the search resumes *after* the matched region.
//! This ensures that a ligature like `->` takes precedence over its prefix `-`,
//! and that no character is consumed by more than one match.
//!
//! # Usage
//!
//! [`LigatureTree`] is constructed once from a set of `(sequence, value)` bindings
//! (typically the full glyph map of a [`RasterFont`]), and then may be queried repeatedly.
//!
//! ```no_run
//! # use raster_font::tree::*;
//! # let font: LigatureTree<()> = todo!();
//!
//! // Resolves all matched glyphs in order, skipping unrecognized characters.
//! for glyph in font.valid("Hello :)") {
//!     // render glyph...
//! }
//! ```
//!
//! For single-match lookup rather than full iteration, see [`LigatureTree::find`].
//!
//! [Aho-Corasick]: https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm
//! [`RasterFont`]: crate::prelude::RasterFont

use aho_corasick::{AhoCorasick, FindIter, Input, Match, MatchKind, StreamFindIter};

#[doc(inline)]
pub use aho_corasick::BuildError;

/// An [Aho-Corasick] automaton configured for leftmost-longest matching of ligature sequences.
///
/// LigatureTrees are typically constructed from a full set of glyph bindings for a font, and then
/// used to resolve glyphs from input strings when rendering text with that font.
///
/// See the [module-level docs](self) for information on matching behavior and usage examples.
///
///
/// # Example
///
/// Create a new `LigatureTree`, then query a single match with [`find`],
/// or iterate over all valid matches sequentially with [`valid`].
///
/// ```rust
/// # use raster_font::prelude::*;
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Glyph {
///   Dash,
///   Arrow,
///   Smile
/// }
/// use Glyph::*;
///
/// let tree = LigatureTree::try_from_bindings([
///     ("-",   Dash),
///     ("->",  Arrow),
///     ("=>",  Arrow),
///     (":)",  Smile),
/// ]).unwrap();
///
/// assert_eq!(tree.find("Hello :) ->"), Some(&Smile));
///
/// let mut iter = tree.valid(r#"Hello :) ->-  =>"#);
/// assert_eq!(iter.next(), Some(&Smile));
/// assert_eq!(iter.next(), Some(&Arrow));
/// assert_eq!(iter.next(), Some(&Dash));
/// assert_eq!(iter.next(), Some(&Arrow));
/// ```
///
/// [Aho-Corasick]: https://en.wikipedia.org/wiki/Aho%E2%80%93Corasick_algorithm
/// [`find`]: Self::find
/// [`valid`]: Self::valid
#[derive(Clone, Debug)]
pub struct LigatureTree<V> {
    /// Aho-Corasick automaton for efficient longest-match-first searching of ligatures.
    aho: aho_corasick::AhoCorasick,
    /// Values associated with each ligature pattern, indexed by the order they were provided when
    /// creating the `LigatureTree`.
    ligatures: Vec<V>,
}

impl<V> LigatureTree<V> {
    /// Create a LigatureTree from an iterator of `(sequence, value)` pairs.
    pub fn try_from_bindings<S: AsRef<[u8]>>(
        bindings: impl IntoIterator<Item = (S, V)>,
    ) -> Result<Self, BuildError> {
        let (patterns, ligatures): (Vec<_>, Vec<_>) = bindings.into_iter().unzip();

        let aho = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(patterns)?;

        Ok(Self { aho, ligatures })
    }

    /// Get a ligature value by its pattern index, which corresponds to the order in which the patterns
    /// were provided when creating the `LigatureTree`.
    pub fn get_by_pattern_index(&self, pattern_index: usize) -> Option<&V> {
        self.ligatures.get(pattern_index)
    }

    /// Returns the glyph of the first, leftmost-longest match in `input`.
    ///
    /// See [`aho_corasick::AhoCorasick::find`] for more details on matching behavior.
    pub fn find(&self, input: impl AsRef<str>) -> Option<&V> {
        let m = self.aho.find(input.as_ref())?;
        self.get_by_pattern_index(m.pattern().as_usize())
    }

    /// Returns an iterator of all valid ligatures in the input string, in the order they appear.
    #[inline]
    pub fn valid<'i>(
        &'_ self,
        input: impl Into<Input<'i>>,
    ) -> LigatureIter<'_, V, FindIter<'_, 'i>> {
        LigatureIter {
            aho_iter: self.aho.find_iter(input),
            ligatures: &self.ligatures,
        }
    }

    /// Returns an iterator of all valid ligatures in the input stream, in the order they appear.
    #[inline]
    pub fn valid_stream<R: std::io::Read>(
        &'_ self,
        stream: R,
    ) -> LigatureStreamIter<'_, V, StreamFindIter<'_, R>> {
        LigatureStreamIter {
            aho_iter: self.aho.stream_find_iter(stream),
            ligatures: &self.ligatures,
        }
    }
}

/// Iterator over valid ligatures in an input.
///
/// Each call to `next` advances past the
/// current match and finds the next leftmost-longest pattern, skipping any
/// unrecognized characters between matches.
///
/// See [`LigatureTree`] for details.
pub struct LigatureIter<'l, V, I>
where
    I: Iterator,
{
    aho_iter: I,
    ligatures: &'l [V],
}

impl<'l, V, I> Iterator for LigatureIter<'l, V, I>
where
    I: Iterator<Item = Match>,
{
    type Item = &'l V;

    fn next(&mut self) -> Option<Self::Item> {
        self.ligatures
            .get(self.aho_iter.next()?.pattern().as_usize())
    }
}

/// Stream iterator over valid ligatures in an input.
///
/// Each call to `next` advances past the
/// current match and finds the next leftmost-longest pattern, skipping any
/// unrecognized characters between matches.
///
/// See [`LigatureTree`] for details.
pub struct LigatureStreamIter<'l, V, I>
where
    I: Iterator,
{
    aho_iter: I,
    ligatures: &'l [V],
}

impl<'l, V, I> Iterator for LigatureStreamIter<'l, V, I>
where
    I: Iterator<Item = Result<Match, std::io::Error>>,
{
    type Item = Result<&'l V, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.aho_iter
            .next()?
            .map(|m| self.ligatures.get(m.pattern().as_usize()))
            .transpose()
    }
}

#[cfg(test)]
#[test]
fn test_ligature_tree() {
    let bindings = vec![
        ("abc", "lig1"),
        ("ab", "lig2"),
        ("bc", "lig3"),
        ("c", "lig4"),
        ("y", "lig5"),
    ];

    let tree = LigatureTree::try_from_bindings(bindings).unwrap();

    assert_eq!(tree.find("abc"), Some(&"lig1"));
    assert_eq!(tree.find("ab"), Some(&"lig2"));
    assert_eq!(tree.find("bc"), Some(&"lig3"));
    assert_eq!(tree.find("c"), Some(&"lig4"));
    assert_eq!(tree.find("d"), None);
    assert_eq!(tree.find("y"), Some(&"lig5"));
    assert_eq!(tree.find("dabcy"), Some(&"lig1"));
    assert_eq!(tree.find("ydabcy"), Some(&"lig5"));

    let valid: Vec<_> = tree.valid("xabcdy").cloned().collect();
    assert_eq!(valid, vec!["lig1", "lig5"]);
}

/// A trait for types that can provide access to an internal [`LigatureTree`].
///
/// Prefer implementing `AsRef<LigatureTree<T>>` directly for your type instead of this trait, as it
/// provides more flexibility and automatically grants `AsLigatureTree` and [`InputResolver`]
/// implementations.
pub trait AsLigatureTree<T> {
    fn as_ligature_tree(&self) -> &LigatureTree<T>;
}

/// Blanket implementation to convert any reference to a LigatureTree into an AsLigatureTree.
impl<T, U: AsRef<LigatureTree<T>>> AsLigatureTree<T> for U {
    #[inline]
    fn as_ligature_tree(&self) -> &LigatureTree<T> {
        self.as_ref()
    }
}

/// Describes how to convert the base ligature tree value (e.g. an AtlasIndex) into the final value
/// used for rendering (e.g. a Glyph).
pub trait MapLigature<T>: AsLigatureTree<T> {
    type Output;

    fn map_ligature(&self, value: &T) -> Self::Output;
}

/// A trait for types that can resolve input strings to ligature values with an internal LigatureTree.
pub trait InputResolver<T>: AsLigatureTree<T> + MapLigature<T> {
    /// Get a ligature value by its pattern index, which corresponds to the order in which the patterns
    /// were provided when creating the `LigatureTree`.
    fn get_by_pattern_index(&self, pattern_index: usize) -> Option<Self::Output> {
        let lig = self
            .as_ligature_tree()
            .get_by_pattern_index(pattern_index)?;
        Some(self.map_ligature(lig))
    }

    /// Returns the glyph of the first, leftmost-longest match in `input`.
    ///
    /// See [`aho_corasick::AhoCorasick::find`] for more details on matching behavior.
    fn find(&self, input: impl AsRef<str>) -> Option<Self::Output> {
        let lig = self.as_ligature_tree().find(input)?;
        Some(self.map_ligature(lig))
    }

    /// Returns an iterator of all valid ligatures in the input string, in the order they appear.
    fn valid<'a, 'i>(&'a self, input: impl Into<Input<'i>>) -> impl Iterator<Item = Self::Output>
    where
        T: 'a,
    {
        self.as_ligature_tree()
            .valid(input)
            .map(|lig| self.map_ligature(lig))
    }

    /// Returns an iterator of all valid ligatures in the input stream, in the order they appear.
    fn valid_stream<'a, R: std::io::Read>(
        &'a self,
        stream: R,
    ) -> impl Iterator<Item = Result<Self::Output, std::io::Error>>
    where
        T: 'a,
    {
        self.as_ligature_tree()
            .valid_stream(stream)
            .map(|res| res.map(|lig| self.map_ligature(lig)))
    }
}

/// Blanket implementation to convert any AsLigatureTree into an InputResolver, which provides
/// convenience methods like `find` and `valid` for querying ligatures directly from the tree.
impl<T, U: AsLigatureTree<T> + MapLigature<T>> InputResolver<T> for U {}
