//! Rust знає лише механічну драбину доменів: точну ширину, payload і зворотне перетворення.
//! Закони resident-ів та значення бітів визначаються Lisp-контрактами SENS.

use sens::{
    Bit1, Bit2, Bit3, Bit4, Bit5, Bit6, Bit7, Bit8, Bit9, BinarySourceWord,
    BitPacker, DomainD10, DomainIdentity, PackedBitstream,
};

fn assert_ladder_round_trip(source: BinarySourceWord, width: usize, payload: u16) {
    assert_eq!(source.width(), width);
    assert_eq!(source.packed_bits(), payload);
    assert_eq!(source.to_string().len(), width);

    let identity = DomainIdentity::from_source_word(source);
    assert_eq!(identity.width(), width);
    assert_eq!(identity.packed_bits(), payload);
    assert_eq!(identity.source_word(), source);
}

#[test]
fn exact_width_ladder_round_trips_payloads_without_truncation() {
    for raw in 0..=1 {
        assert_ladder_round_trip(BinarySourceWord::W1(Bit1::new(raw).unwrap()), 1, raw as u16);
    }
    for raw in 0..=3 {
        assert_ladder_round_trip(BinarySourceWord::W2(Bit2::new(raw).unwrap()), 2, raw as u16);
    }
    for raw in 0..=7 {
        assert_ladder_round_trip(BinarySourceWord::W3(Bit3::new(raw).unwrap()), 3, raw as u16);
    }
    for raw in 0..=15 {
        assert_ladder_round_trip(BinarySourceWord::W4(Bit4::new(raw).unwrap()), 4, raw as u16);
    }
    for raw in 0..=31 {
        assert_ladder_round_trip(BinarySourceWord::W5(Bit5::new(raw).unwrap()), 5, raw as u16);
    }
    for raw in 0..=63 {
        assert_ladder_round_trip(BinarySourceWord::W6(Bit6::new(raw).unwrap()), 6, raw as u16);
    }
    for raw in 0..=127 {
        assert_ladder_round_trip(BinarySourceWord::W7(Bit7::new(raw).unwrap()), 7, raw as u16);
    }
    for raw in 0..=255 {
        assert_ladder_round_trip(BinarySourceWord::W8(Bit8::new(raw).unwrap()), 8, raw as u16);
    }
    for raw in 0..=511 {
        assert_ladder_round_trip(BinarySourceWord::W9(Bit9::new(raw).unwrap()), 9, raw);
    }
}

#[test]
fn same_payload_at_different_widths_keeps_distinct_domain_identity() {
    let d1 = BinarySourceWord::W1(Bit1::new(1).unwrap()).domain_identity();
    let d2 = BinarySourceWord::W2(Bit2::new(1).unwrap()).domain_identity();
    let d3 = BinarySourceWord::W3(Bit3::new(1).unwrap()).domain_identity();

    assert_eq!(d1.packed_bits(), d2.packed_bits());
    assert_eq!(d2.packed_bits(), d3.packed_bits());
    assert_ne!(d1, d2);
    assert_ne!(d2, d3);
    assert_ne!(d1, d3);
}

#[test]
fn width_carriers_reject_payloads_that_do_not_fit() {
    assert!(Bit1::new(2).is_none());
    assert!(Bit2::new(4).is_none());
    assert!(Bit3::new(8).is_none());
    assert!(Bit4::new(16).is_none());
    assert!(Bit5::new(32).is_none());
    assert!(Bit6::new(64).is_none());
    assert!(Bit7::new(128).is_none());
    // W8's u8 carrier can express every 8-bit payload; W9 must reject 512.
    assert!(Bit9::new(512).is_none());
}

#[test]
fn d10_public_carrier_preserves_all_ten_bits_without_ast_admission() {
    use sens::{
        domain_ladder::DomainCoordinate, parse_canonical_binary,
        parse_domain_ladder_source_words,
    };

    // Exhaust the public type, and independently confirm the source/coordinate boundary.
    for bits in 0u16..1024 {
        let d10 = DomainD10::new(bits).expect("every ten-bit payload is representable");
        assert_eq!(d10.packed_bits(), bits);

        let coordinate = d10.coordinate();
        assert_eq!(coordinate.width(), 10);
        assert_eq!(coordinate.bits(), bits);
        assert_eq!(DomainD10::from_coordinate(coordinate), Some(d10));
        assert_eq!(coordinate.owner_residency(), Some(true));

        let source = format!("{bits:010b}");
        let parsed = parse_domain_ladder_source_words(&source)
            .expect("D1-D10 source-coordinate path must preserve ten exact bits");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].coordinate, coordinate);

        // Same payload does not erase the D9/D10 width boundary.
        if bits < 512 {
            assert_ne!(coordinate, DomainCoordinate::new(9, bits).unwrap());
        }
    }

    for source in ["0000000000", "0111111111", "1000000000", "1111111111"] {
        assert!(
            parse_canonical_binary(source).is_err(),
            "legacy AST admission must not silently promote D10 source {source}"
        );
    }
    assert!(DomainD10::new(1024).is_none());
    assert!(DomainD10::new(u16::MAX).is_none());
    assert!(DomainD10::from_coordinate(DomainCoordinate::new(9, 511).unwrap()).is_none());
}

#[test]
fn d10_uses_ten_dense_bits_across_physical_byte_boundaries() {
    use sens::domain_ladder::DomainCoordinate;
    use std::mem::size_of;

    let prefix = DomainCoordinate::new(3, 0b101).unwrap();
    let d10 = DomainD10::new(0b1010101010).unwrap();
    let suffix = DomainCoordinate::new(4, 0b0110).unwrap();

    let mut packer = BitPacker::new();
    assert_eq!(packer.push_coordinate(prefix), 0);
    assert_eq!(packer.push_coordinate(d10.coordinate()), 3);
    assert_eq!(packer.push_coordinate(suffix), 13);
    let packed = packer.finish();

    // Independent 17-bit MSB-first golden vector: 101 | 1010101010 | 0110.
    assert_eq!(packed.bytes(), &[0xB5, 0x53, 0x00]);
    assert_eq!(packed.bit_len(), 17);
    assert_eq!(packed.byte_len(), 3);
    assert_eq!(packed.valid_bits_in_last_byte(), 1);
    assert_eq!(packed.read_coordinate(0, 3), Some(prefix));
    assert_eq!(packed.read_coordinate(3, 10), Some(d10.coordinate()));
    assert_eq!(packed.read_coordinate(13, 4), Some(suffix));

    // Host storage is two bytes; transport payload is exactly ten bits.
    assert_eq!(size_of::<DomainD10>(), 2);
    assert!(PackedBitstream::from_parts(vec![0xB5, 0x53, 0x01], 17).is_none());
}

