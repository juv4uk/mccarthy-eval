use sens::{BinarySourceWord, Bit2, Bit7, BitPacker, Text7, Text7W7Error, Text7WordError};

#[test]
fn text7_dense_w7_round_trips_without_interior_padding() {
    let cells = vec![0x00, 0x0f, 0x14, 0x18, 0x3e, 0x3f, 0x5c, 0x5d];
    let text = Text7::from_cells(cells.clone()).unwrap();

    let packed = text.to_packed_w7();
    assert_eq!(packed.bit_len(), 56);
    assert_eq!(packed.byte_len(), 7);

    let recovered = Text7::from_packed_w7(&packed).unwrap();
    assert_eq!(recovered, text);
    assert_eq!(recovered.cells(), &cells[..]);

    // Canonical Text7 wire authority is unchanged by the W7 mechanism.
    assert_eq!(recovered.to_canonical_wire_token(), text.to_canonical_wire_token());
}

#[test]
fn arbitrary_cell_count_pays_only_final_byte_slack() {
    let text = Text7::from_cells(vec![0x00, 0x11, 0x16]).unwrap();
    let packed = text.to_packed_w7();

    assert_eq!(packed.bit_len(), 21);
    assert_eq!(packed.byte_len(), 3);
}

#[test]
fn packed_non_multiple_of_seven_fails_closed() {
    let mut packer = BitPacker::new();
    packer.push(Bit7::new(0x15).unwrap());
    packer.push(Bit2::new(0b10).unwrap());
    let packed = packer.finish();

    assert_eq!(packed.bit_len(), 9);
    assert_eq!(
        Text7::from_packed_w7(&packed).unwrap_err(),
        Text7W7Error::UnalignedBitLen { bit_len: 9 }
    );
}

#[test]
fn source_words_require_exact_w7_and_preserve_cells() {
    let text = Text7::from_cells(vec![0x00, 0x24, 0x44]).unwrap();
    let words = text.to_source_words();

    assert_eq!(words.len(), 3);
    assert!(words.iter().all(|word| matches!(word, BinarySourceWord::W7(_))));
    assert_eq!(Text7::from_source_words(&words).unwrap(), text);

    let bad = [
        BinarySourceWord::W7(Bit7::new(1).unwrap()),
        BinarySourceWord::W2(Bit2::new(2).unwrap()),
    ];
    assert_eq!(
        Text7::from_source_words(&bad).unwrap_err(),
        Text7WordError::InvalidWordWidth { index: 1, width: 2 }
    );
}

#[test]
fn w7_is_a_mechanical_width_not_sound_admission() {
    let word = BinarySourceWord::W7(Bit7::new(1).unwrap());
    assert_eq!(word.width(), 7);

    // Explicit construction of Text7 is the semantic wrapper. Equal payload
    // bits alone do not collapse the W7 source word into Text7 identity.
    let text = Text7::from_cells(vec![1]).unwrap();
    assert_eq!(word.packed_bits(), u16::from(text.cells()[0]));
    assert_eq!(text.to_canonical_wire_token(), "#t7:01");
}

#[test]
fn exact_d7_carrier_never_mints_a_callable_operation() {
    use sens::DomainIdentity;
    for raw in 0u8..=127 {
        let source = BinarySourceWord::W7(Bit7::new(raw).unwrap());
        let identity = DomainIdentity::from_source_word(source);
        assert_eq!(identity.width(), 7);
        assert_eq!(identity.packed_bits(), raw as u16);
        assert_eq!(identity.source_word(), source);
        assert_eq!(identity.core_operation(), None);
    }
}

#[test]
fn d7_w7_full_payload_is_dense() {
    let data = Text7::from_cells((0u8..=127).collect::<Vec<u8>>()).unwrap();
    let packed = data.to_packed_w7();
    assert_eq!(packed.bit_len(), 896);
    assert_eq!(packed.byte_len(), 112);
    assert_eq!(Text7::from_packed_w7(&packed).unwrap(), data);
}
