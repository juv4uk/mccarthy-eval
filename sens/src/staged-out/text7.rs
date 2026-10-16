use std::{fmt, rc::Rc};

use crate::{BinarySourceWord, Bit7, BitPacker, PackedBitstream};

/// Canonical SENS D7 v2 text identity: exact, typed 7-bit coordinates.
///
/// The current keyboard profile has 128 ASCII-indexed coordinates. The
/// en/uk/san Unicode views are projections, not alternate machine identities.
/// A transient host byte per cell does not turn D7 into historical SID8.
///
/// Human Unicode spelling is deliberately absent from canonical equality,
/// dense transport, and the legacy #t7: token. The ASCII keyboard projection
/// alone is an exact identity conversion; Ukrainian/Devanagari rendering
/// belongs to the separately versioned boundary projector.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Text7 {
    cells: Rc<[u8]>,
}

/// Failure to admit a host byte as one exact UPC-7 cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Text7CellError {
    index: usize,
    byte: u8,
}

impl Text7 {
    pub const LOGICAL_WIDTH: u8 = 7;
    pub const MAX_CELL: u8 = 0b0111_1111;

    /// Domain tag of the canonical machine transport token. The tag keeps a
    /// Text7 value distinct from Number (`#q2:`/`#b`), from a String spelling
    /// and from the bare eight-bit Function8 space.
    pub const WIRE_TAG: &'static str = "#t7:";

    /// Admit a sequence only when every mechanism byte is exactly one UPC-7
    /// cell. No masking or truncation is allowed: a set high bit is a named
    /// failure, never an implicit modulo-128 conversion.
    pub fn from_cells(cells: impl Into<Vec<u8>>) -> Result<Self, Text7CellError> {
        let cells = cells.into();
        if let Some((index, &byte)) = cells
            .iter()
            .enumerate()
            .find(|(_, byte)| **byte > Self::MAX_CELL)
        {
            return Err(Text7CellError { index, byte });
        }

        Ok(Self {
            cells: cells.into(),
        })
    }

    /// Admit the owner-ratified D7 v2 *English keyboard* projection.
    ///
    /// ASCII position 0..=127 is already the exact W7 coordinate. UTF-8
    /// multibyte glyphs require a separately selected uk/san projection:
    /// interpreting their bytes as new D7 cells would silently corrupt
    /// the program. The first offending input byte is reported, not masked.
    pub fn from_ascii_projection(source: &str) -> Result<Self, Text7CellError> {
        Self::from_cells(source.as_bytes().to_vec())
    }

    /// Display W7 cells through the English ASCII projection only.
    ///
    /// This is a human boundary view, never a new canonical wire encoding.
    /// Other script projections must not be inferred from these glyphs.
    pub fn to_ascii_projection(&self) -> String {
        self.cells.iter().copied().map(char::from).collect()
    }

    /// Canonical machine transport for one Text7 value.
    ///
    /// The token carries the exact UPC-7 cell stream: the domain tag followed
    /// by two lowercase hex digits per cell. It is deliberately not a human
    /// spelling — no Unicode, no UTF-8 and no selected layout appears in it —
    /// and `to_canonical_wire_string` must never fall back to human `render`
    /// for this domain.
    pub fn to_canonical_wire_token(&self) -> String {
        let mut token = String::with_capacity(Self::WIRE_TAG.len() + self.cells.len() * 2);
        token.push_str(Self::WIRE_TAG);
        for cell in self.cells.iter() {
            token.push(char::from_digit((cell >> 4) as u32, 16).expect("nibble is a hex digit"));
            token.push(char::from_digit((cell & 0x0F) as u32, 16).expect("nibble is a hex digit"));
        }
        token
    }

    /// Admit a canonical Text7 wire token back to its exact cell stream.
    ///
    /// Fails closed on every malformed shape: a missing tag, an odd digit
    /// count, a non-hex (or non-lowercase) digit, or a cell with the forbidden
    /// high bit set is a named error — never a masked, truncated or
    /// Unicode-transcoded value.
    pub fn from_canonical_wire_token(token: &str) -> Result<Self, Text7WireError> {
        let payload = token
            .strip_prefix(Self::WIRE_TAG)
            .ok_or(Text7WireError::MissingTag)?;
        if payload.len() % 2 != 0 {
            return Err(Text7WireError::OddLength {
                digits: payload.len(),
            });
        }

        let mut cells = Vec::with_capacity(payload.len() / 2);
        for (pair_index, pair) in payload.as_bytes().chunks(2).enumerate() {
            let high = hex_nibble(pair[0]).ok_or(Text7WireError::NonHexDigit {
                index: pair_index * 2,
                byte: pair[0],
            })?;
            let low = hex_nibble(pair[1]).ok_or(Text7WireError::NonHexDigit {
                index: pair_index * 2 + 1,
                byte: pair[1],
            })?;
            let byte = (high << 4) | low;
            if byte > Self::MAX_CELL {
                return Err(Text7WireError::CellOutOfRange {
                    index: pair_index,
                    byte,
                });
            }
            cells.push(byte);
        }

        Ok(Self {
            cells: cells.into(),
        })
    }

    /// Exact UPC-7 cell stream.
    ///
    /// The returned bytes are storage only; callers must not reinterpret them
    /// as SENS Numbers or widen Text identity beyond seven bits.
    pub fn cells(&self) -> &[u8] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Pack this exact Text7 cell sequence as one dense homogeneous W7 payload.
    ///
    /// This is mechanism only: it preserves seven payload bits per cell with
    /// zero interior byte padding. Framing and semantic admission remain
    /// separate concerns.
    pub fn to_packed_w7(&self) -> PackedBitstream {
        let mut packer = BitPacker::with_capacity_bits(self.cells.len() * 7);
        for &cell in self.cells.iter() {
            packer.push(Bit7::new(cell).expect("Text7 already enforces exact seven-bit cells"));
        }
        packer.finish()
    }

    /// Reconstruct Text7 from a homogeneous dense W7 payload.
    ///
    /// A payload whose exact bit length is not divisible by seven is rejected
    /// rather than padded, truncated, or guessed.
    pub fn from_packed_w7(packed: &PackedBitstream) -> Result<Self, Text7W7Error> {
        if !packed.bit_len().is_multiple_of(7) {
            return Err(Text7W7Error::UnalignedBitLen {
                bit_len: packed.bit_len(),
            });
        }

        let count = packed.bit_len() / 7;
        let mut cells = Vec::with_capacity(count);
        for index in 0..count {
            let word = packed
                .read::<7>(index * 7)
                .expect("validated W7 offset remains inside payload");
            cells.push(word.packed_bits());
        }
        Self::from_cells(cells).map_err(|_| {
            unreachable!("Bit7 read cannot produce a value wider than seven bits")
        })
    }

    /// Project every Text7 cell to an exact W7 source word.
    ///
    /// Width is explicit in the enum variant; this does not turn arbitrary W7
    /// source words into SoundCell semantics.
    pub fn to_source_words(&self) -> Vec<BinarySourceWord> {
        self.cells
            .iter()
            .map(|&cell| {
                BinarySourceWord::W7(
                    Bit7::new(cell).expect("Text7 already enforces exact seven-bit cells"),
                )
            })
            .collect()
    }

    /// Reconstruct Text7 only from an all-W7 source-word sequence.
    ///
    /// Any other exact-width word is a named failure. No width coercion occurs.
    pub fn from_source_words(words: &[BinarySourceWord]) -> Result<Self, Text7WordError> {
        let mut cells = Vec::with_capacity(words.len());
        for (index, &word) in words.iter().enumerate() {
            match word {
                BinarySourceWord::W7(word) => cells.push(word.packed_bits()),
                other => {
                    return Err(Text7WordError::InvalidWordWidth {
                        index,
                        width: other.width(),
                    });
                }
            }
        }
        Self::from_cells(cells).map_err(|_| {
            unreachable!("Bit7 source word cannot contain a value wider than seven bits")
        })
    }
}

impl TryFrom<Vec<u8>> for Text7 {
    type Error = Text7CellError;

    fn try_from(cells: Vec<u8>) -> Result<Self, Self::Error> {
        Self::from_cells(cells)
    }
}

impl TryFrom<&[u8]> for Text7 {
    type Error = Text7CellError;

    fn try_from(cells: &[u8]) -> Result<Self, Self::Error> {
        Self::from_cells(cells.to_vec())
    }
}

impl Text7CellError {
    pub fn index(self) -> usize {
        self.index
    }

    /// Offending host storage byte. This is diagnostic mechanism data, not a
    /// Text7 identity.
    pub fn packed_byte(self) -> u8 {
        self.byte
    }
}

/// Failure to admit a canonical Text7 wire token. Every failure is named;
/// nothing here silently converts a malformed token into Text identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Text7WireError {
    /// The token does not carry the `#t7:` domain tag.
    MissingTag,
    /// The payload is not an even-length sequence of cell bytes.
    OddLength { digits: usize },
    /// A payload character is not a lowercase hex digit.
    NonHexDigit { index: usize, byte: u8 },
    /// A cell byte sets the forbidden high bit (`>= 128`).
    CellOutOfRange { index: usize, byte: u8 },
}

/// Only canonical lowercase hex is admitted; uppercase is a named failure so
/// one cell stream cannot gain two canonical spellings.
fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

impl fmt::Display for Text7CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Text7 cell {} must fit exactly seven bits; got {:08b}",
            self.index, self.byte
        )
    }
}

impl std::error::Error for Text7CellError {}

/// Failure when decoding a homogeneous packed W7 payload as Text7.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Text7W7Error {
    UnalignedBitLen { bit_len: usize },
}

impl fmt::Display for Text7W7Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnalignedBitLen { bit_len } => write!(
                formatter,
                "packed W7 payload length {bit_len} is not divisible by 7"
            ),
        }
    }
}

impl std::error::Error for Text7W7Error {}

/// Failure when reconstructing Text7 from exact-width source words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Text7WordError {
    InvalidWordWidth { index: usize, width: usize },
}

impl fmt::Display for Text7WordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWordWidth { index, width } => write!(
                formatter,
                "source word at index {index} has width {width}; Text7 requires exact W7"
            ),
        }
    }
}

impl std::error::Error for Text7WordError {}

impl fmt::Debug for Text7 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Text7(")?;
        for (index, cell) in self.cells.iter().enumerate() {
            if index != 0 {
                f.write_str(" ")?;
            }
            write!(f, "{cell:07b}")?;
        }
        f.write_str(")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_only_exact_seven_bit_cells() {
        let text = Text7::from_cells(vec![0b0000000, 0b0000001, 0b1111111]).unwrap();

        assert_eq!(text.cells(), &[0, 1, 127]);
        assert_eq!(text.len(), 3);
        assert!(!text.is_empty());
        assert_eq!(format!("{text:?}"), "Text7(0000000 0000001 1111111)");
    }

    #[test]
    fn rejects_first_cell_with_a_high_bit_instead_of_masking_it() {
        let error = Text7::from_cells(vec![0b0000001, 0b10000000, 0b1111111]).unwrap_err();

        assert_eq!(error.index(), 1);
        assert_eq!(error.packed_byte(), 0b10000000);
        assert!(error.to_string().contains("10000000"));

        let max_error = Text7::from_cells(vec![0xff]).unwrap_err();
        assert_eq!(max_error.index(), 0);
        assert_eq!(max_error.packed_byte(), 0xff);
    }

    #[test]
    fn equality_is_exact_code_stream_equality() {
        let left = Text7::from_cells(vec![0b0000001, 0b0000010]).unwrap();
        let same = Text7::from_cells(vec![0b0000001, 0b0000010]).unwrap();
        let different = Text7::from_cells(vec![0b0000001, 0b0000011]).unwrap();

        assert_eq!(left, same);
        assert_ne!(left, different);
    }

    /// One test covers every D7 v2 keyboard position, including C0 and DEL:
    /// 128 W7 values are exactly 896 meaningful bits or 112 physical bytes.
    #[test]
    fn d7_v2_ascii_128_exact_coordinates_without_sid8_widening() {
        let ascii: String = (0u8..=127).map(char::from).collect();
        let cells = Text7::from_ascii_projection(&ascii).expect("128 exact W7 coordinates");
        assert_eq!(cells.cells(), &(0u8..=127).collect::<Vec<_>>());
        assert_eq!(cells.to_ascii_projection(), ascii);
        assert_eq!(cells.to_source_words().len(), 128);
        assert!(cells.to_source_words().iter().all(
            |word| matches!(word, BinarySourceWord::W7(_))
        ));
        let packed = cells.to_packed_w7();
        assert_eq!(packed.bit_len(), 128 * 7);
        assert_eq!(packed.byte_len(), 112);
        assert_eq!(Text7::from_packed_w7(&packed).unwrap(), cells);
        // Same host numeric value in W8 is NOT a W7 coordinate.
        let old = BinarySourceWord::W8(crate::Bit8::new(0b1101001).unwrap());
        assert_eq!(
            Text7::from_source_words(&[old]).unwrap_err(),
            Text7WordError::InvalidWordWidth { index: 0, width: 8 }
        );
    }

    /// Independent physical bytes for one genuine Lisp binding "identity".
    /// This is not merely an encode->decode round-trip; wrong MSB-first
    /// packing would pass a symmetric codec but fail these golden vectors.
    #[test]
    fn d7_v2_ascii_lisp_identifier_golden_and_bad_padding_fail_closed() {
        let name = Text7::from_ascii_projection("identity").unwrap();
        assert_eq!(name.cells(), b"identity");
        let physical = name.to_packed_w7();
        assert_eq!(physical.bit_len(), 56);
        assert_eq!(physical.byte_len(), 7);
        assert_eq!(physical.bytes(), &[0xd3, 0x93, 0x2e, 0xee, 0x9a, 0x7a, 0x79]);
        assert_eq!(Text7::from_packed_w7(&physical).unwrap().to_ascii_projection(), "identity");

        let unaligned = Text7::from_ascii_projection("value").unwrap().to_packed_w7();
        assert_eq!(unaligned.bit_len(), 35);
        assert_eq!(unaligned.bytes(), &[0xed, 0x87, 0x67, 0x5c, 0xa0]);
        assert_eq!(unaligned.valid_bits_in_last_byte(), 3);
        let mut corrupt = unaligned.bytes().to_vec();
        corrupt[4] |= 0b0000_0001;
        assert!(PackedBitstream::from_parts(corrupt, 35).is_none());
        let wrong_length = PackedBitstream::from_parts(unaligned.bytes().to_vec(), 36)
            .expect("36 bits can form an intact generic bitstream");
        assert_eq!(
            Text7::from_packed_w7(&wrong_length).unwrap_err(),
            Text7W7Error::UnalignedBitLen { bit_len: 36 }
        );
    }

    #[test]
    fn d7_v2_unicode_requires_explicit_non_ascii_keyboard_projection() {
        for source in ["ї", "ґ", "क", "🙂", "aї"] {
            let error = Text7::from_ascii_projection(source).unwrap_err();
            let index = source.as_bytes().iter().position(|byte| *byte >= 128).unwrap();
            assert_eq!(error.index(), index);
            assert_eq!(error.packed_byte(), source.as_bytes()[index]);
        }
        assert_eq!(Text7::from_ascii_projection("a'b").unwrap().to_ascii_projection(), "a'b");
        assert_eq!(Text7::from_ascii_projection("").unwrap().len(), 0);
    }

    #[test]
    fn empty_text_is_a_valid_empty_code_stream() {
        let text = Text7::from_cells(Vec::new()).unwrap();

        assert!(text.is_empty());
        assert_eq!(text.cells(), &[] as &[u8]);
        assert_eq!(format!("{text:?}"), "Text7()");
    }
}
