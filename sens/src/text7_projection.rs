//! Human text data through the current ratified D7 v2 keyboard coordinates.
//! A quoted function identity must resolve through the domain ladder first;
//! this projection is for text DATA only and never confers callability.

use crate::{d7_keyboard, Text7};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Text7Layout {
    Uk,
    SaSlp1,
    SaIast,
    SaDeva,
    En,
    EnIpa,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Text7ProjectionError {
    UnknownSpelling { layout: Text7Layout, byte_offset: usize },
    UnassignedSpelling {
        layout: Text7Layout,
        byte_offset: usize,
        spelling: &'static str,
    },
    UnrenderableCell { layout: Text7Layout, index: usize, cell: u8 },
    AmbiguousGlyphSequence { layout: Text7Layout },
}

/// Only the actual v2 keyboard surfaces are mappings. The former phonemic
/// SLP1, IAST and IPA aliases are never guessed from unrelated v2 coordinates.
fn current_mode(layout: Text7Layout) -> Option<d7_keyboard::D7Mode> {
    match layout {
        Text7Layout::Uk => Some(d7_keyboard::D7Mode::Uk),
        Text7Layout::SaDeva => Some(d7_keyboard::D7Mode::San),
        Text7Layout::En => Some(d7_keyboard::D7Mode::En),
        Text7Layout::SaSlp1 | Text7Layout::SaIast | Text7Layout::EnIpa => None,
    }
}

/// The selected text projection must be known before a W7 data stream is made.
/// D2 punctuation is structural only in the *source grammar*, not in a text value.
pub fn encode_text7(text: &str, layout: Text7Layout) -> Result<Text7, Text7ProjectionError> {
    let mode = current_mode(layout).ok_or(Text7ProjectionError::UnknownSpelling {
        layout, byte_offset: 0,
    })?;
    let words = d7_keyboard::encode(text, mode).map_err(|err| {
        let byte_offset = match err {
            d7_keyboard::D7CodecError::CharacterNotAdmitted(ch) =>
                text.char_indices().find(|(_, candidate)| *candidate == ch)
                    .map(|(offset, _)| offset).unwrap_or(0),
            _ => 0,
        };
        Text7ProjectionError::UnknownSpelling { layout, byte_offset }
    })?;
    Text7::from_cells(words).map_err(|_| Text7ProjectionError::UnknownSpelling {
        layout, byte_offset: 0,
    })
}

pub fn render_text7(text: &Text7, layout: Text7Layout) -> Result<String, Text7ProjectionError> {
    // Human-only IAST display is generated from the same 128 exact W7 cells.
    // Digraphs and dependent vowel markers cannot be decoded as a single
    // scalar Sanskrit keyboard code: SaIast render does NOT grant encode.
    if layout == Text7Layout::SaIast {
        let mut out = String::new();
        for (index, &cell) in text.cells().iter().enumerate() {
            let word = crate::BinarySourceWord::W7(
                crate::Bit7::new(cell).ok_or(
                    Text7ProjectionError::UnrenderableCell { layout, index, cell }
                )?
            );
            let identity = crate::DomainIdentity::from_source_word(word);
            let glyph = crate::semantic_registry::surface_for_domain_identity(
                identity, "sa"
            ).ok_or(Text7ProjectionError::UnrenderableCell {
                layout, index, cell,
            })?;
            out.push_str(glyph);
        }
        return Ok(out);
    }
    let mode = current_mode(layout).ok_or(Text7ProjectionError::UnknownSpelling {
        layout, byte_offset: 0,
    })?;
    d7_keyboard::decode(text.cells(), mode).map_err(|_| {
        Text7ProjectionError::UnrenderableCell {
            layout, index: 0, cell: text.cells().first().copied().unwrap_or(0),
        }
    })
}

impl fmt::Display for Text7ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSpelling { layout, byte_offset } =>
                write!(f, "{layout:?} has no current D7 v2 glyph at byte {byte_offset}"),
            Self::UnassignedSpelling { layout, byte_offset, spelling } =>
                write!(f, "{layout:?} unsupported {spelling:?} at byte {byte_offset}"),
            Self::UnrenderableCell { layout, index, cell } =>
                write!(f, "{layout:?} cannot display W7:{cell:07b} at index {index}"),
            Self::AmbiguousGlyphSequence { layout } =>
                write!(f, "{layout:?} loses exact W7 cell boundaries"),
        }
    }
}
impl std::error::Error for Text7ProjectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_128_cells_roundtrip_under_each_owner_v2_projection() {
        let exact: Vec<u8> = (0..=127).collect();
        let datum = Text7::from_cells(exact).unwrap();
        for layout in [Text7Layout::Uk, Text7Layout::SaDeva, Text7Layout::En] {
            let shown = render_text7(&datum, layout).unwrap();
            assert_eq!(encode_text7(&shown, layout).unwrap(), datum);
        }
    }

    #[test]
    fn ukrainian_uppercase_and_punctuation_are_single_w7_cells() {
        let phrase = "Привіт! Кіт, №1 ₴";
        let words = encode_text7(phrase, Text7Layout::Uk).unwrap();
        assert_eq!(words.cells().len(), phrase.chars().count());
        assert_eq!(render_text7(&words, Text7Layout::Uk).unwrap(), phrase);
        assert_eq!(encode_text7("!", Text7Layout::Uk).unwrap().cells(), &[33]);
        assert_eq!(encode_text7("*", Text7Layout::Uk).unwrap().cells(), &[42]);
    }

    #[test]
    fn no_legacy_phonetic_profile_is_used_as_v2_keyboard() {
        for layout in [Text7Layout::SaSlp1, Text7Layout::SaIast, Text7Layout::EnIpa] {
            assert!(matches!(encode_text7("k", layout),
                Err(Text7ProjectionError::UnknownSpelling { .. })));
        }
    }

    #[test]
    fn iast_renders_exact_d7_cells_without_claiming_ambiguous_inverse() {
        let cells = Text7::from_cells(vec![75, 107, 69, 101]).unwrap();
        assert_eq!(render_text7(&cells, Text7Layout::SaIast).unwrap(), "khkā◌ā");
        assert!(matches!(
            encode_text7("khkā◌ā", Text7Layout::SaIast),
            Err(Text7ProjectionError::UnknownSpelling { .. })
        ));
        // Original ratified Devanagari v2 codec remains explicitly versioned.
        assert_eq!(render_text7(&cells, Text7Layout::SaDeva).unwrap(), "खकआा");
    }

    #[test]
    fn exact_width_and_unassigned_unicode_fail_closed() {
        assert!(matches!(
            encode_text7("🙂", Text7Layout::Uk),
            Err(Text7ProjectionError::UnknownSpelling { .. })
        ));
        assert_ne!(
            encode_text7("0", Text7Layout::Uk).unwrap(),
            encode_text7("00", Text7Layout::Uk).unwrap()
        );
    }
}
