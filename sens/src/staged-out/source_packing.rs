//! ZIG-OWNED: physical bit-packing is the sterile Zig corridor's job (t5/f3c/transport.zig).
//! Rust is byte-addressed and must NOT pack: Zig hands it already-packed programs.
//! This module is retained only for the staged removal (#5617/#5609). Do not extend it.
//! Mechanical bridge from binary source words to the ZIG corridor.
//! ZIG-OWNED: the physical packing is the sterile Zig corridor's job (zig_transport / transport_abi.zig).
//! These functions delegate to the Zig ABI; Rust does not pack.
//!
//! This module assigns no semantic roles and defines no wire framing. It only
//! preserves source order while appending each exact-width word into one
//! continuous PackedBitstream.
//!
//! Important: a PackedBitstream preserves payload bits and total bit length,
//! not source word boundaries. Distinct exact-width source sequences can have
//! the same packed payload; a grammar or explicit framing layer must supply
//! widths when reconstruction requires them.

use crate::{domain_ladder::DomainCoordinate, BinarySourceToken, BinarySourceWord, BitPacker, PackedBitstream};

/// Accounting for one canonical densely-packed semantic payload.
///
/// The payload carries only exact semantic bits. `framing_bits` is supplied by
/// the caller because grammar/EOS/container framing is a separate protocol
/// concern. `tail_unused_bits` is physical byte-container slack only; it is
/// never semantic padding and never appears between words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackedTransportAccounting {
    pub semantic_payload_bits: usize,
    pub framing_bits: usize,
    pub tail_unused_bits: usize,
    pub total_wire_bits: usize,
}

impl PackedTransportAccounting {
    /// Fraction of physical wire bits that are semantic payload.
    ///
    /// Empty transport has no utilization ratio.
    pub fn utilization(self) -> Option<f64> {
        (self.total_wire_bits != 0)
            .then(|| self.semantic_payload_bits as f64 / self.total_wire_bits as f64)
    }
}

/// Report semantic payload, external framing and final physical-byte slack
/// independently.
///
/// This function deliberately does not invent a width schedule or framing
/// format. A caller that already owns grammar/context can pass `framing_bits=0`;
/// a standalone container can pass its independently-accounted framing cost.
pub fn packed_transport_accounting(
    packed: &PackedBitstream,
    framing_bits: usize,
) -> PackedTransportAccounting {
    let physical_payload_bits = packed.byte_len() * 8;
    let tail_unused_bits = physical_payload_bits - packed.bit_len();
    PackedTransportAccounting {
        semantic_payload_bits: packed.bit_len(),
        framing_bits,
        tail_unused_bits,
        total_wire_bits: physical_payload_bits + framing_bits,
    }
}

/// Pack exact D1–D10 coordinates with no interior byte padding.
///
/// Width is part of each validated coordinate. This operation does not infer
/// a domain from a numeric byte, assign callable semantics, or define a
/// standalone physical wire format; the decoder requires an external schedule.
pub fn pack_domain_coordinates(coordinates: &[DomainCoordinate]) -> PackedBitstream {
    let total_bits: usize = coordinates
        .iter()
        .map(|coordinate| usize::from(coordinate.width()))
        .sum();
    let mut packer = BitPacker::with_capacity_bits(total_bits);
    for &coordinate in coordinates {
        packer.push_coordinate(coordinate);
    }
    packer.finish()
}

/// Reconstruct exact D1–D10 coordinates using a caller-owned width schedule.
///
/// Reject invalid width, overflow, and incomplete or overlong schedules.
/// W8 and D8 remain separate identity types; this function never interprets
/// a byte as a historical function or a callable current resident.
pub fn unpack_domain_coordinates(
    packed: &PackedBitstream,
    widths: &[u8],
) -> Option<Vec<DomainCoordinate>> {
    let expected_bits = widths.iter().try_fold(0usize, |total, &width| {
        if !(1..=10).contains(&width) {
            return None;
        }
        total.checked_add(usize::from(width))
    })?;
    if expected_bits != packed.bit_len() {
        return None;
    }

    let mut coordinates = Vec::with_capacity(widths.len());
    let mut offset = 0usize;
    for &width in widths {
        coordinates.push(packed.read_coordinate(offset, width)?);
        offset += usize::from(width);
    }
    debug_assert_eq!(offset, packed.bit_len());
    Some(coordinates)
}

/// Append one already-bounded source word to the current dense payload.
///
/// Returns the starting bit offset of the word in the packed payload.
pub fn append_binary_source_word(packer: &mut BitPacker, word: BinarySourceWord) -> usize {
    // Width belongs to the already-validated source word, not the host u16.
    // Use the same bit-exact packer as D1–D10 without assigning any callability.
    let coordinate = DomainCoordinate::new(word.width() as u8, word.packed_bits())
        .expect("a typed W1–W9 source word must fit its exact width");
    packer.push_coordinate(coordinate)
}

/// Count the exact semantic payload bits carried by canonical source tokens.
///
/// This function performs no semantic role lookup and invents no domain width.
/// Each token already carries its exact SENS source width; Rust only adds those
/// widths. Source whitespace/comments are absent from `tokens`, while D2
/// structural words remain ordinary exact-width source tokens and therefore
/// contribute their bits.
///
/// Wire framing and final-byte slack are intentionally excluded.
pub fn semantic_source_bits(tokens: &[BinarySourceToken]) -> usize {
    tokens.iter().map(|token| token.word.width()).sum()
}

/// Pack exact source words in program order with zero interior byte padding.
///
/// Source token spans and semantic word-boundary policy stay outside the packed
/// payload. The returned value is deliberately not self-describing with respect
/// to source word widths. Canonical wire/EOS and boundary metadata are owned
/// separately by framing work.
fn pack_source_words<I>(words: I, total_bits: usize) -> PackedBitstream
where
    I: IntoIterator<Item = BinarySourceWord>,
{
    let mut packer = BitPacker::with_capacity_bits(total_bits);
    for word in words {
        append_binary_source_word(&mut packer, word);
    }
    packer.finish()
}

/// Pack already-decoded exact-width words directly, without constructing a
/// visible zero/one source string or reparsing its tokens.
pub fn pack_binary_source_words(words: &[BinarySourceWord]) -> PackedBitstream {
    let total_bits = words.iter().map(|word| word.width()).sum();
    pack_source_words(words.iter().copied(), total_bits)
}

/// Pack exact source tokens in program order with zero interior byte padding.
/// This retains the source-token adapter for callers that already own spans.
pub fn pack_binary_source_tokens(tokens: &[BinarySourceToken]) -> PackedBitstream {
    pack_source_words(
        tokens.iter().map(|token| token.word),
        semantic_source_bits(tokens),
    )
}

fn read_binary_source_word(
    packed: &PackedBitstream,
    bit_offset: usize,
    width: usize,
) -> Option<BinarySourceWord> {
    Some(match width {
        1 => BinarySourceWord::W1(packed.read::<1>(bit_offset)?),
        2 => BinarySourceWord::W2(packed.read::<2>(bit_offset)?),
        3 => BinarySourceWord::W3(packed.read::<3>(bit_offset)?),
        4 => BinarySourceWord::W4(packed.read::<4>(bit_offset)?),
        5 => BinarySourceWord::W5(packed.read::<5>(bit_offset)?),
        6 => BinarySourceWord::W6(packed.read::<6>(bit_offset)?),
        7 => BinarySourceWord::W7(packed.read::<7>(bit_offset)?),
        8 => BinarySourceWord::W8(packed.read::<8>(bit_offset)?),
        9 => BinarySourceWord::W9(packed.read_w9(bit_offset)?),
        _ => return None,
    })
}

/// Reconstruct exact source words from a dense payload when the caller already
/// owns the exact word-width schedule.
///
/// This is deliberately grammar/domain-bound mechanics, not a standalone wire
/// decoder and not semantic dispatch. Equal-total alternative width schedules
/// can describe the same raw payload bits, so `widths` is explicit
/// caller-owned boundary information. Standalone self-description belongs to
/// the framing layer.
///
/// The decode fails closed if any width is outside 1..=9 or if the supplied
/// widths do not consume the payload bit length exactly.
pub fn unpack_binary_source_words(
    packed: &PackedBitstream,
    widths: &[usize],
) -> Option<Vec<BinarySourceWord>> {
    let expected_bits = widths.iter().try_fold(0usize, |total, &width| {
        if !(1..=9).contains(&width) {
            return None;
        }
        total.checked_add(width)
    })?;

    if expected_bits != packed.bit_len() {
        return None;
    }

    let mut words = Vec::with_capacity(widths.len());
    let mut bit_offset = 0usize;

    for &width in widths {
        words.push(read_binary_source_word(packed, bit_offset, width)?);
        bit_offset += width;
    }

    debug_assert_eq!(bit_offset, packed.bit_len());
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_binary_source_words;

    /// Вичерпний бітовий свідок: 2+4+8+...+512 = 1022 слова.
    ///
    /// Порівнює мігрований W1–W9 source шлях із єдиним D1–D10 packer,
    /// перевіряючи також нульові старші біти, offsets і фізичний roundtrip.
    #[test]
    fn all_1022_exact_d1_to_d9_words_use_one_lossless_dense_packer() {
        let mut source = String::new();
        let mut coordinates = Vec::new();
        let mut expected_bits = 0usize;
        for width in 1u8..=9 {
            for bits in 0..(1u16 << width) {
                source.push_str(&format!("{bits:0width$b} ", width = usize::from(width)));
                coordinates.push(DomainCoordinate::new(width, bits).unwrap());
                expected_bits += usize::from(width);
            }
        }
        assert_eq!(coordinates.len(), 1022);

        let tokens = parse_binary_source_words(&source).unwrap();
        assert_eq!(tokens.len(), coordinates.len());
        let mut packer = BitPacker::with_capacity_bits(expected_bits);
        let mut bit_offset = 0usize;
        for (token, &coordinate) in tokens.iter().zip(&coordinates) {
            assert_eq!(token.word.width(), usize::from(coordinate.width()));
            assert_eq!(token.word.packed_bits(), coordinate.bits());
            assert_eq!(append_binary_source_word(&mut packer, token.word), bit_offset);
            bit_offset += usize::from(coordinate.width());
        }

        let source_packed = packer.finish();
        let domain_packed = pack_domain_coordinates(&coordinates);
        assert_eq!(source_packed.bit_len(), expected_bits);
        assert!(source_packed == domain_packed, "all exact-width bits and total length must match");
        assert_eq!(source_packed.byte_len(), expected_bits.div_ceil(8));

        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        let domain_widths: Vec<_> = coordinates.iter().map(|coordinate| coordinate.width()).collect();
        assert_eq!(
            unpack_binary_source_words(&source_packed, &widths),
            Some(tokens.iter().map(|token| token.word).collect()),
        );
        assert_eq!(
            unpack_domain_coordinates(&source_packed, &domain_widths),
            Some(coordinates),
        );
    }

    #[test]
    fn d9_high_bit_and_leading_zeros_never_alias_eight_bit_source() {
        let words = parse_binary_source_words(
            "000000001 100000001 111111111 00000001",
        ).unwrap();
        let packed = pack_binary_source_tokens(&words);
        assert_eq!(packed.bit_len(), 35);
        assert_eq!(
            unpack_binary_source_words(&packed, &[9, 9, 9, 8]),
            Some(words.iter().map(|token| token.word).collect()),
        );
        // D9 high bit is an identity bit, not disposable physical padding.
        assert_ne!(
            DomainCoordinate::new(9, 0b100000001),
            DomainCoordinate::new(8, 0b00000001),
        );
        assert_eq!(DomainCoordinate::new(9, 1).unwrap().width(), 9);
        assert_eq!(DomainCoordinate::new(9, 0b1_0000_0000).unwrap().bits(), 256);
        assert!(DomainCoordinate::new(9, 0b10_0000_0000).is_none());
        assert!(unpack_domain_coordinates(&packed, &[9, 9, 8, 8]).is_none());
    }

    #[test]
    fn domain_ladder_is_packed_at_exact_width_not_host_register_width() {
        let coordinates: Vec<_> = (1..=10u8)
            .map(|width| DomainCoordinate::new(width, (1u16 << width) - 1).unwrap())
            .collect();
        let packed = pack_domain_coordinates(&coordinates);
        let widths: Vec<_> = coordinates.iter().map(|coordinate| coordinate.width()).collect();
        assert_eq!(packed.bit_len(), 55);
        assert_eq!(packed.byte_len(), 7);
        assert_eq!(packed.valid_bits_in_last_byte(), 7);
        assert_eq!(packed.bytes()[6] & 1, 0);
        assert_eq!(unpack_domain_coordinates(&packed, &widths), Some(coordinates));
    }

    #[test]
    fn d1_d9_domain_coordinates_preserve_source_packer_bytes() {
        let tokens = parse_binary_source_words(
            "1 10 101 1010 10101 101010 1010101 10101010 100000001",
        ).unwrap();
        let coordinates: Vec<_> = tokens.iter().map(|token| {
            DomainCoordinate::from_domain_identity(token.word.domain_identity())
        }).collect();
        let packed = pack_domain_coordinates(&coordinates);
        assert!(packed == pack_binary_source_tokens(&tokens), "source and domain packers must emit identical physical bits");
        assert_eq!(packed.bit_len(), 45);
        assert_eq!(packed.byte_len(), 6);
        let widths: Vec<_> = coordinates.iter().map(|coordinate| coordinate.width()).collect();
        assert_eq!(unpack_domain_coordinates(&packed, &widths), Some(coordinates));
    }

    #[test]
    fn d10_high_bits_survive_cross_byte_without_aliasing_d8_d9() {
        let coordinates = [
            DomainCoordinate::new(7, 0b1010101).unwrap(),
            DomainCoordinate::new(10, 0b11_1010_1000).unwrap(),
            DomainCoordinate::new(8, 0b1010_1000).unwrap(),
            DomainCoordinate::new(9, 0b1_1010_1000).unwrap(),
        ];
        let packed = pack_domain_coordinates(&coordinates);
        assert_eq!(packed.bit_len(), 34);
        assert_eq!(packed.byte_len(), 5);
        assert_eq!(
            unpack_domain_coordinates(&packed, &[7, 10, 8, 9]),
            Some(coordinates.to_vec())
        );
        assert_ne!(coordinates[1], coordinates[2]);
        assert_ne!(coordinates[1], coordinates[3]);
    }

    #[test]
    fn exact_domain_decoder_rejects_invalid_or_incomplete_width_schedule() {
        let coordinates = [
            DomainCoordinate::new(3, 0b101).unwrap(),
            DomainCoordinate::new(10, 0b11_0000_0001).unwrap(),
        ];
        let packed = pack_domain_coordinates(&coordinates);
        assert_eq!(packed.bit_len(), 13);
        assert!(unpack_domain_coordinates(&packed, &[0, 10, 3]).is_none());
        assert!(unpack_domain_coordinates(&packed, &[3, 11]).is_none());
        assert!(unpack_domain_coordinates(&packed, &[3]).is_none());
        assert!(unpack_domain_coordinates(&packed, &[3, 9]).is_none());
        assert!(unpack_domain_coordinates(&packed, &[3, 10, 1]).is_none());
        assert_eq!(unpack_domain_coordinates(&packed, &[3, 10]), Some(coordinates.to_vec()));
        let empty = pack_domain_coordinates(&[]);
        assert_eq!(empty.bit_len(), 0);
        assert_eq!(unpack_domain_coordinates(&empty, &[]), Some(vec![]));
    }

    #[test]
    fn raw_payload_cannot_determine_domain_boundaries() {
        let a = [DomainCoordinate::new(1, 0).unwrap(), DomainCoordinate::new(2, 0).unwrap()];
        let b = [DomainCoordinate::new(3, 0).unwrap()];
        let packed = pack_domain_coordinates(&a);
        assert!(packed == pack_domain_coordinates(&b), "equal bits with distinct schedules share payload, not identity");
        assert_eq!(unpack_domain_coordinates(&packed, &[1, 2]), Some(a.to_vec()));
        assert_eq!(unpack_domain_coordinates(&packed, &[3]), Some(b.to_vec()));
        assert_ne!(a[0], b[0]);
    }

    #[test]
    fn semantic_source_measure_counts_d2_structure_and_d3_word() {
        let tokens = parse_binary_source_words("10 001 01").unwrap();

        assert_eq!(semantic_source_bits(&tokens), 7);
        assert_eq!(tokens.iter().map(|token| token.word.width()).collect::<Vec<_>>(), [2, 3, 2]);
    }

    #[test]
    fn semantic_source_measure_matches_dense_payload_across_d1_d9() {
        let tokens = parse_binary_source_words(
            "1 10 101 1010 10101 101010 1010101 10101010 100000001"
        ).unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(semantic_source_bits(&tokens), 45);
        assert_eq!(packed.bit_len(), semantic_source_bits(&tokens));
        assert_eq!(packed.byte_len(), 6);
    }

    #[test]
    fn decoded_words_pack_without_visible_source_reconstruction() {
        let tokens = parse_binary_source_words("10 001 01").unwrap();
        let words: Vec<_> = tokens.iter().map(|token| token.word).collect();
        let widths: Vec<_> = words.iter().map(|word| word.width()).collect();

        let packed = pack_binary_source_words(&words);
        let unpacked = unpack_binary_source_words(&packed, &widths).unwrap();

        assert_eq!(packed.bit_len(), 7);
        assert_eq!(packed.bytes(), &[0b1000_1010]);
        assert_eq!(unpacked, words);
    }

    #[test]
    fn canonical_source_example_becomes_one_dense_seven_bit_payload() {
        let tokens = parse_binary_source_words("10 001 01").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(semantic_source_bits(&tokens), 7);
        assert_eq!(packed.bit_len(), 7);
        assert_eq!(packed.byte_len(), 1);
        assert_eq!(packed.bytes(), &[0b1000_1010]);
        assert_eq!(packed.valid_bits_in_last_byte(), 7);
    }

    #[test]
    fn equal_numeric_values_at_distinct_source_widths_pack_without_alignment() {
        let tokens = parse_binary_source_words("1 01 001 0001").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(tokens.iter().map(|t| t.word.width()).collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!(packed.bit_len(), 10);
        assert_eq!(packed.bytes(), &[0b1010_0100, 0b0100_0000]);
    }

    #[test]
    fn source_word_may_cross_physical_byte_boundary() {
        let tokens = parse_binary_source_words("1111111 101").unwrap();
        let mut packer = BitPacker::new();

        let first = append_binary_source_word(&mut packer, tokens[0].word);
        let crossing = append_binary_source_word(&mut packer, tokens[1].word);
        let packed = packer.finish();

        assert_eq!(first, 0);
        assert_eq!(crossing, 7);
        assert_eq!(packed.bit_len(), 10);
        assert_eq!(packed.bytes(), &[0b1111_1111, 0b0100_0000]);
    }

    #[test]
    fn structural_bit_patterns_are_packed_as_payload_not_transport_delimiters() {
        let tokens = parse_binary_source_words("00 01 10 11").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(packed.bit_len(), 8);
        assert_eq!(packed.bytes(), &[0b0001_1011]);
    }

    #[test]
    fn mixed_widths_one_to_eight_reach_the_payload_lower_bound() {
        let tokens =
            parse_binary_source_words("1 10 101 1010 10101 101010 1010101 10101010").unwrap();
        let payload_bits = semantic_source_bits(&tokens);
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(payload_bits, 36);
        assert_eq!(packed.bit_len(), payload_bits);
        assert_eq!(packed.byte_len(), payload_bits.div_ceil(8));
        assert_eq!(packed.bytes(), &[0b1101_0110, 0b1010_1011, 0b0101_0101, 0b0101_1010, 0b1010_0000]);
    }

    #[test]
    fn grammar_bound_w9_round_trips_without_byte_aliasing() {
        let source = "100000001 000000001 111111111";
        let tokens = parse_binary_source_words(source).unwrap();
        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        assert_eq!(widths, [9, 9, 9]);

        let packed = pack_binary_source_tokens(&tokens);
        assert_eq!(packed.bit_len(), 27);
        assert_eq!(packed.byte_len(), 4);

        let decoded = unpack_binary_source_words(&packed, &widths).unwrap();
        assert_eq!(decoded, exact_words(&tokens));
        assert_eq!(decoded[0].packed_bits(), 257);
        assert_eq!(decoded[1].packed_bits(), 1);
        assert_eq!(decoded[2].packed_bits(), 511);
    }

    #[test]
    fn packed_payload_does_not_claim_source_word_boundaries() {
        let split = parse_binary_source_words("0 00").unwrap();
        let single = parse_binary_source_words("000").unwrap();

        assert_eq!(
            split.iter().map(|token| token.word.width()).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            single.iter().map(|token| token.word.width()).collect::<Vec<_>>(),
            [3]
        );

        let split_packed = pack_binary_source_tokens(&split);
        let single_packed = pack_binary_source_tokens(&single);

        assert_eq!(split_packed.bit_len(), 3);
        assert_eq!(single_packed.bit_len(), 3);
        assert_eq!(split_packed.bytes(), single_packed.bytes());
        assert_eq!(split_packed.bytes(), &[0b0000_0000]);
    }

    fn exact_words(tokens: &[BinarySourceToken]) -> Vec<BinarySourceWord> {
        tokens.iter().map(|token| token.word).collect()
    }

    fn assert_grammar_bound_round_trip(source: &str) {
        let tokens = parse_binary_source_words(source).unwrap();
        let widths: Vec<_> = tokens.iter().map(|token| token.word.width()).collect();
        let packed = pack_binary_source_tokens(&tokens);

        let decoded = unpack_binary_source_words(&packed, &widths).expect("exact widths decode");
        assert_eq!(decoded, exact_words(&tokens));
    }

    #[test]
    fn grammar_bound_decoder_round_trips_canonical_example() {
        assert_grammar_bound_round_trip("10 001 01");
    }

    #[test]
    fn grammar_bound_decoder_preserves_width_anti_collapse() {
        assert_grammar_bound_round_trip("1 01 001 0001");
    }

    #[test]
    fn grammar_bound_decoder_handles_cross_byte_and_all_widths() {
        assert_grammar_bound_round_trip("1111111 101");
        assert_grammar_bound_round_trip("1 10 101 1010 10101 101010 1010101 10101010");
    }

    #[test]
    fn grammar_bound_decoder_rejects_invalid_or_non_exact_total_widths() {
        let tokens = parse_binary_source_words("10 001 01").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert!(unpack_binary_source_words(&packed, &[0, 3, 2]).is_none());
        assert!(unpack_binary_source_words(&packed, &[10]).is_none());
        assert!(unpack_binary_source_words(&packed, &[2, 3]).is_none());
        assert!(unpack_binary_source_words(&packed, &[2, 3, 3]).is_none());
    }

    #[test]
    fn equal_total_width_schedule_is_explicit_caller_owned_information() {
        let split = parse_binary_source_words("0 00").unwrap();
        let single = parse_binary_source_words("000").unwrap();
        let packed = pack_binary_source_tokens(&split);

        assert!(packed == pack_binary_source_tokens(&single));
        assert_eq!(
            unpack_binary_source_words(&packed, &[1, 2]).unwrap(),
            exact_words(&split)
        );
        assert_eq!(
            unpack_binary_source_words(&packed, &[3]).unwrap(),
            exact_words(&single)
        );
    }

    #[test]
    fn eight_d1_values_are_one_physical_byte() {
        let tokens = parse_binary_source_words("1 1 1 1 1 1 1 1").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(packed.bit_len(), 8);
        assert_eq!(packed.byte_len(), 1);
        assert_eq!(packed.bytes(), &[0b1111_1111]);
        assert_eq!(packed.valid_bits_in_last_byte(), 8);
    }

    #[test]
    fn canonical_transport_accounting_keeps_payload_framing_and_tail_separate() {
        let tokens = parse_binary_source_words("10 001 01").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        let grammar_bound = packed_transport_accounting(&packed, 0);
        assert_eq!(
            grammar_bound,
            PackedTransportAccounting {
                semantic_payload_bits: 7,
                framing_bits: 0,
                tail_unused_bits: 1,
                total_wire_bits: 8,
            }
        );
        assert_eq!(grammar_bound.utilization(), Some(7.0 / 8.0));

        let standalone = packed_transport_accounting(&packed, 5);
        assert_eq!(standalone.semantic_payload_bits, 7);
        assert_eq!(standalone.framing_bits, 5);
        assert_eq!(standalone.tail_unused_bits, 1);
        assert_eq!(standalone.total_wire_bits, 13);
        assert_eq!(standalone.utilization(), Some(7.0 / 13.0));
    }

    #[test]
    fn homogeneous_exact_width_blocks_pay_no_interior_or_tail_padding() {
        for (source, expected_bits, expected_bytes) in [
            ("10101 10101 10101 10101 10101 10101 10101 10101", 40, 5),
            ("101010 101010 101010 101010", 24, 3),
            ("1010101 1010101 1010101 1010101 1010101 1010101 1010101 1010101", 56, 7),
        ] {
            let tokens = parse_binary_source_words(source).unwrap();
            let packed = pack_binary_source_tokens(&tokens);
            let accounting = packed_transport_accounting(&packed, 0);

            assert_eq!(accounting.semantic_payload_bits, expected_bits);
            assert_eq!(packed.byte_len(), expected_bytes);
            assert_eq!(accounting.tail_unused_bits, 0);
            assert_eq!(accounting.total_wire_bits, expected_bits);
            assert_eq!(accounting.utilization(), Some(1.0));
        }
    }

    #[test]
    fn comments_and_whitespace_do_not_consume_payload_bits() {
        let tokens = parse_binary_source_words("10 ; open\n 001\t01 ; close\n").unwrap();
        let packed = pack_binary_source_tokens(&tokens);

        assert_eq!(semantic_source_bits(&tokens), 7);
        assert_eq!(packed.bit_len(), 7);
        assert_eq!(packed.bytes(), &[0b1000_1010]);
    }
}
