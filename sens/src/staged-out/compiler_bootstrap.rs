//! Bootstrap-only mechanical view of exact domain identity.
//!
//! This module deliberately exposes representation, not SENS meaning.
//! It must never map a coordinate to CAR/CDR/CONS, a compiler role, or a
//! backend mechanism.  The returned function value is opt-in: embedders pass
//! it to SENS code explicitly while #3808/#3809 move compiler-law execution
//! into the language itself.

use crate::{sha256_source, ErrorKind, Exactness, LanguageError, Value};
use std::rc::Rc;

/// Build an opt-in mechanism value that decomposes one exact DomainIdentity.
///
/// Result:
///
///     (width (bit0 bit1 ... bitN))
///
/// Bits are exact D1 PredicateBit values in source order (MSB first).
/// Width is a host numeric representation fact, not a semantic-domain role.
///
/// This function does not register a global surface or mint a SENS identity.
pub fn domain_identity_shape_mechanism() -> Value {
    Value::host_function(Rc::new(|arguments, _environment, span| {
        if arguments.len() != 1 {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!(
                    "domain identity shape mechanism expects exactly 1 argument, got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let identity = arguments[0].as_domain_identity().ok_or_else(|| {
            LanguageError::new(
                ErrorKind::Type,
                "domain identity shape mechanism expects an exact DomainIdentity",
                span,
            )
        })?;

        let width = identity.width();
        let packed = identity.packed_bits();

        let bits = (0..width).map(|index| {
            let shift = width - 1 - index;
            Value::predicate_bit(((packed >> shift) & 1) == 1)
        });

        Ok(Value::list([
            Value::Number(width as f64, Exactness::Exact),
            Value::list(bits),
        ]))
    }))
}

/// Build an opt-in representation predicate/decomposer for compiler program-data.
///
/// For an exact DomainIdentity this returns the same `(width bits)` shape as
/// `domain_identity_shape_mechanism`. Every other value returns NIL instead
/// of raising a type error. This is intentionally representation-only: it
/// does not admit a domain, choose a compiler role, select proof, or choose a
/// backend mechanism.
pub fn domain_identity_shape_or_empty_mechanism() -> Value {
    Value::host_function(Rc::new(|arguments, _environment, span| {
        if arguments.len() != 1 {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!(
                    "domain identity shape-or-empty mechanism expects exactly 1 argument, got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let Some(identity) = arguments[0].as_domain_identity() else {
            return Ok(Value::Nil);
        };

        let width = identity.width();
        let packed = identity.packed_bits();
        let bits = (0..width).map(|index| {
            let shift = width - 1 - index;
            Value::predicate_bit(((packed >> shift) & 1) == 1)
        });

        Ok(Value::list([
            Value::Number(width as f64, Exactness::Exact),
            Value::list(bits),
        ]))
    }))
}

fn put_canonical_compiler_len(out: &mut Vec<u8>, len: usize) {
    out.extend_from_slice(&(len as u64).to_le_bytes());
}

fn encode_canonical_compiler_value(value: &Value, out: &mut Vec<u8>) -> Result<(), String> {
    match value {
        Value::Nil => out.push(0x00),
        Value::DomainIdentity(identity) => {
            out.push(0x01);
            out.push(identity.width() as u8);
            if identity.width() <= 8 {
                // Preserve existing compiler-hash bytes for D1-D8.
                out.push(identity.packed_bits() as u8);
            } else {
                // W9 must never be truncated to the historical byte lane.
                out.extend_from_slice(&identity.packed_bits().to_le_bytes());
            }
        }
        Value::Symbol(symbol) => {
            out.push(0x02);
            put_canonical_compiler_len(out, symbol.len());
            out.extend_from_slice(symbol.as_bytes());
        }
        Value::String(text) => {
            out.push(0x03);
            put_canonical_compiler_len(out, text.len());
            out.extend_from_slice(text.as_bytes());
        }
        Value::Pair(head, tail) => {
            out.push(0x04);
            encode_canonical_compiler_value(head, out)?;
            encode_canonical_compiler_value(tail, out)?;
        }
        other => {
            return Err(format!(
                "canonical compiler-value hash does not admit runtime value: {other}"
            ));
        }
    }
    Ok(())
}

/// Canonical binary representation for proof-carrying compiler evidence.
///
/// This is the exact representation hashed by `canonical_value_sha256_mechanism`.
/// It admits only NIL, exact DomainIdentity, Symbol, String and Pair, and owns
/// no compiler meaning or backend policy.
pub fn compiler_evidence_canonical_bytes(value: &Value) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    encode_canonical_compiler_value(value, &mut encoded)?;
    Ok(encoded)
}


const MAX_CANONICAL_COMPILER_DEPTH: usize = 4096;

struct CanonicalCompilerDecoder<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> CanonicalCompilerDecoder<'a> {
    fn take_byte(&mut self) -> Result<u8, String> {
        let byte = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| "truncated canonical compiler evidence".to_string())?;
        self.offset += 1;
        Ok(byte)
    }

    fn take_len(&mut self) -> Result<usize, String> {
        let end = self
            .offset
            .checked_add(8)
            .ok_or_else(|| "canonical compiler evidence length overflow".to_string())?;
        let raw = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| "truncated canonical compiler evidence length".to_string())?;
        self.offset = end;
        let len = u64::from_le_bytes(
            raw.try_into()
                .expect("canonical compiler evidence length is exactly eight bytes"),
        );
        usize::try_from(len)
            .map_err(|_| "canonical compiler evidence length does not fit usize".to_string())
    }

    fn take_utf8(&mut self, label: &str) -> Result<Rc<str>, String> {
        let len = self.take_len()?;
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| format!("{label} length overflow"))?;
        let raw = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| format!("truncated canonical compiler {label}"))?;
        self.offset = end;
        let text = std::str::from_utf8(raw)
            .map_err(|_| format!("canonical compiler {label} is not valid UTF-8"))?;
        Ok(Rc::from(text))
    }

    fn decode_domain_identity(&mut self) -> Result<Value, String> {
        use crate::{
            BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, DomainIdentity,
        };

        let width = self.take_byte()?;
        // Keep the D1–D8 single-byte digest ABI; D9 is exactly two
        // little-endian bytes, as already written by the encoder.
        // D10 has no Value::DomainIdentity carrier here.
        let packed = match width {
            1..=8 => u16::from(self.take_byte()?),
            9 => {
                let low = self.take_byte()?;
                let high = self.take_byte()?;
                u16::from_le_bytes([low, high])
            }
            _ => return Err(format!(
                "canonical compiler domain identity width must be 1..=9, got {width}"
            )),
        };
        let invalid = || {
            format!(
                "canonical compiler domain identity has invalid D{width} payload {packed}"
            )
        };
        let source = match width {
            1 => BinarySourceWord::W1(Bit1::new(packed as u8).ok_or_else(invalid)?),
            2 => BinarySourceWord::W2(Bit2::new(packed as u8).ok_or_else(invalid)?),
            3 => BinarySourceWord::W3(Bit3::new(packed as u8).ok_or_else(invalid)?),
            4 => BinarySourceWord::W4(Bit4::new(packed as u8).ok_or_else(invalid)?),
            5 => BinarySourceWord::W5(Bit5::new(packed as u8).ok_or_else(invalid)?),
            6 => BinarySourceWord::W6(Bit6::new(packed as u8).ok_or_else(invalid)?),
            7 => BinarySourceWord::W7(Bit7::new(packed as u8).ok_or_else(invalid)?),
            8 => BinarySourceWord::W8(Bit8::new(packed as u8).ok_or_else(invalid)?),
            9 => BinarySourceWord::W9(crate::Bit9::new(packed).ok_or_else(invalid)?),
            _ => unreachable!("width checked before reading payload"),
        };
        Ok(Value::DomainIdentity(DomainIdentity::from_source_word(source)))
    }

    fn decode_value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_CANONICAL_COMPILER_DEPTH {
            return Err("canonical compiler evidence nesting exceeds limit".to_string());
        }

        match self.take_byte()? {
            0x00 => Ok(Value::Nil),
            0x01 => self.decode_domain_identity(),
            0x02 => Ok(Value::Symbol(self.take_utf8("symbol")?)),
            0x03 => Ok(Value::String(self.take_utf8("string")?)),
            0x04 => {
                let head = self.decode_value(depth + 1)?;
                let tail = self.decode_value(depth + 1)?;
                Ok(Value::Pair(Rc::new(head), Rc::new(tail)))
            }
            tag => Err(format!(
                "unknown canonical compiler evidence tag 0x{tag:02x}"
            )),
        }
    }
}

/// Decode the exact SENS-owned canonical compiler-evidence representation.
///
/// This is representation-only and is the strict inverse of
/// `compiler_evidence_canonical_bytes`. It reconstructs exact-width domain
/// identities without assigning any language role or backend mechanism.
pub fn compiler_evidence_from_canonical_bytes(bytes: &[u8]) -> Result<Value, String> {
    let mut decoder = CanonicalCompilerDecoder { bytes, offset: 0 };
    let value = decoder.decode_value(0)?;
    if decoder.offset != bytes.len() {
        return Err(format!(
            "canonical compiler evidence has {} trailing bytes",
            bytes.len() - decoder.offset
        ));
    }
    Ok(value)
}

/// Narrow representation-only SHA-256 over compiler-evidence values.
///
/// SENS chooses the value to hash. The mechanism admits only the ordinary
/// immutable data needed by the proof-carrying compiler request sequence:
/// NIL, exact DomainIdentity, Symbol, String and Pair. It owns no domain
/// meaning, role/proof routing or backend policy.
pub fn canonical_value_sha256_mechanism() -> Value {
    Value::host_function(Rc::new(|arguments, _environment, span| {
        if arguments.len() != 1 {
            return Err(LanguageError::new(
                ErrorKind::Arity,
                format!(
                    "canonical-value sha256 mechanism expects exactly 1 argument, got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let encoded = compiler_evidence_canonical_bytes(&arguments[0])
            .map_err(|message| LanguageError::new(ErrorKind::Type, message, span))?;
        let digest = sha256_source(&encoded)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(Value::String(Rc::from(digest)))
    }))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_evidence_round_trips_exact_value_and_width() {
        use crate::{
            BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, DomainIdentity,
        };

        let identities = [
            DomainIdentity::from_source_word(BinarySourceWord::W1(Bit1::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W2(Bit2::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W3(Bit3::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W4(Bit4::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W5(Bit5::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W6(Bit6::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W7(Bit7::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W8(Bit8::new(1).unwrap())),
            DomainIdentity::from_source_word(BinarySourceWord::W9(crate::Bit9::new(1).unwrap())),
        ];

        let value = Value::list(
            identities
                .into_iter()
                .map(Value::DomainIdentity)
                .chain([
                    Value::Symbol(Rc::from("compiler-compilation-artifact/1")),
                    Value::String(Rc::from("точні-bytes")),
                ]),
        );
        let encoded = compiler_evidence_canonical_bytes(&value).unwrap();
        let decoded = compiler_evidence_from_canonical_bytes(&encoded).unwrap();
        assert_eq!(decoded, value);

        let mut cursor = decoded;
        for expected_width in 1..=9 {
            let Value::Pair(head, tail) = &cursor else {
                panic!("round-trip list must retain all exact-width identities");
            };
            let Value::DomainIdentity(identity) = head.as_ref() else {
                panic!("round-trip item must remain an exact DomainIdentity");
            };
            assert_eq!(identity.width(), expected_width);
            assert_eq!(identity.packed_bits(), 1);
            cursor = tail.as_ref().clone();
        }
    }


    /// Exact D1–D9 compiler evidence = 1022 independently addressed values.
    /// All D10 coordinates (1024) must stay outside this Value ABI:
    /// 2046 coordinates are NOT 2046 executable functions.
    #[test]
    fn compiler_evidence_exhaustive_2046_coordinates_exact_width() {
        use crate::{
            BinarySourceWord, Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9,
            DomainIdentity,
        };
        use crate::domain_ladder::DomainCoordinate;

        let mut checked = 0usize;
        for width in 1u8..=9 {
            for bits in 0u16..(1u16 << width) {
                let source = match width {
                    1 => BinarySourceWord::W1(Bit1::new(bits as u8).unwrap()),
                    2 => BinarySourceWord::W2(Bit2::new(bits as u8).unwrap()),
                    3 => BinarySourceWord::W3(Bit3::new(bits as u8).unwrap()),
                    4 => BinarySourceWord::W4(Bit4::new(bits as u8).unwrap()),
                    5 => BinarySourceWord::W5(Bit5::new(bits as u8).unwrap()),
                    6 => BinarySourceWord::W6(Bit6::new(bits as u8).unwrap()),
                    7 => BinarySourceWord::W7(Bit7::new(bits as u8).unwrap()),
                    8 => BinarySourceWord::W8(Bit8::new(bits as u8).unwrap()),
                    9 => BinarySourceWord::W9(Bit9::new(bits).unwrap()),
                    _ => unreachable!(),
                };
                let identity = DomainIdentity::from_source_word(source);
                let value = Value::DomainIdentity(identity);
                let encoded = compiler_evidence_canonical_bytes(&value).unwrap();
                assert_eq!(&encoded[..2], &[0x01, width]);
                if width == 9 {
                    assert_eq!(encoded.len(), 4);
                    assert_eq!(&encoded[2..], &bits.to_le_bytes());
                } else {
                    assert_eq!(encoded.len(), 3, "D{width} historical hash ABI");
                    assert_eq!(encoded[2], bits as u8);
                }
                let decoded = compiler_evidence_from_canonical_bytes(&encoded)
                    .unwrap_or_else(|error| panic!("D{width} {bits:b}: {error}"));
                assert_eq!(decoded, value, "D{width} {bits:b}");
                checked += 1;
            }
        }
        assert_eq!(checked, 1022);

        for bits in 0u16..1024 {
            let coordinate = DomainCoordinate::new(10, bits).unwrap();
            assert_eq!(coordinate.width(), 10);
            let [low, high] = bits.to_le_bytes();
            assert!(
                compiler_evidence_from_canonical_bytes(&[0x01, 10, low, high]).is_err(),
                "D10 {bits:010b} must not silently truncate into D9/W8"
            );
        }
    }

    #[test]
    fn compiler_evidence_decoder_fails_closed_on_malformed_bytes() {
        assert!(compiler_evidence_from_canonical_bytes(&[0xff]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x00, 0x00]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x03, 0x08]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x09, 0xff]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x09, 0xff, 0x02]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x09, 0x00, 0x01, 0x00]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x01, 0x0a, 0x00, 0x00]).is_err());
        assert!(compiler_evidence_from_canonical_bytes(&[0x02, 0x01]).is_err());

        let mut invalid_utf8 = vec![0x03];
        invalid_utf8.extend_from_slice(&1u64.to_le_bytes());
        invalid_utf8.push(0xff);
        assert!(compiler_evidence_from_canonical_bytes(&invalid_utf8).is_err());

        let mut trailing = compiler_evidence_canonical_bytes(&Value::Nil).unwrap();
        trailing.push(0x00);
        assert!(compiler_evidence_from_canonical_bytes(&trailing).is_err());
    }

    #[test]
    fn compiler_evidence_binary_layout_is_stable() {
        let identity = crate::CoreDomainIdentity::D3(
            crate::Bija3::from_word(crate::Bit3::new(0b010).expect("D3 word")),
        );
        let value = Value::list([
            Value::DomainIdentity(identity.into()),
            Value::Symbol(Rc::from("x")),
            Value::String(Rc::from("ok")),
        ]);

        let encoded =
            compiler_evidence_canonical_bytes(&value).expect("compiler evidence encoding");

        assert_eq!(
            encoded,
            vec![
                0x04,
                0x01, 0x03, 0x02,
                0x04,
                0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, b'x',
                0x04,
                0x03, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, b'o', b'k',
                0x00,
            ],
            "compiler evidence encoding is a cross-substrate digest ABI"
        );
    }
}
