//! Current ratified D7 v2 keyboard *data* W7 packing and source identities.
//! Historical Shiva-sutra sound assignments must not be imported as current D7.

use sens::{
    encode_text7, render_text7, BinarySourceWord, Bit7, BitPacker,
    Text7, Text7Layout, Text7W7Error, Text7WordError, Value,
};

#[test]
fn current_ukrainian_text_packs_densely_as_seven_bits_per_character() {
    let datum = encode_text7("код", Text7Layout::Uk).unwrap();
    assert_eq!(datum.len(), 3);
    let packed = datum.to_packed_w7();
    assert_eq!(packed.bit_len(), 21);
    assert_eq!(packed.byte_len(), 3);
    let restored = Text7::from_packed_w7(&packed).unwrap();
    assert_eq!(restored, datum);
    assert_eq!(render_text7(&restored, Text7Layout::Uk).unwrap(), "код");
}

#[test]
fn eight_current_text_cells_use_exactly_seven_dense_bytes() {
    let datum = encode_text7("Привіт! ", Text7Layout::Uk).unwrap();
    assert_eq!(datum.len(), 8);
    let packed = datum.to_packed_w7();
    assert_eq!(packed.bit_len(), 56);
    assert_eq!(packed.byte_len(), 7);
    let restored = Text7::from_packed_w7(&packed).unwrap();
    assert_eq!(render_text7(&restored, Text7Layout::Uk).unwrap(), "Привіт! ");
}

#[test]
fn all_exact_w7_cells_preserve_width_via_source_words() {
    let datum = Text7::from_cells((0u8..=127).collect::<Vec<_>>()).unwrap();
    let words = datum.to_source_words();
    assert_eq!(words.len(), 128);
    for word in &words {
        assert_eq!(word.width(), 7);
        assert!(matches!(word, BinarySourceWord::W7(_)));
    }
    assert_eq!(Text7::from_source_words(&words).unwrap(), datum);
    let invalid = vec![
        BinarySourceWord::W7(Bit7::new(0).unwrap()),
        BinarySourceWord::W2(sens::Bit2::new(0b10).unwrap()),
    ];
    assert_eq!(
        Text7::from_source_words(&invalid).unwrap_err(),
        Text7WordError::InvalidWordWidth { index: 1, width: 2 }
    );
}

#[test]
fn keyboard_d7_is_not_an_arithmetic_number_or_callable_function() {
    let exclamation = encode_text7("!", Text7Layout::Uk).unwrap();
    assert_eq!(exclamation.cells(), &[33]);
    assert_eq!(exclamation.to_canonical_wire_token(), "#t7:21");
    assert_ne!(Value::Text7(exclamation), Value::Nil);
    let w7 = sens::DomainIdentity::from_source_word(
        BinarySourceWord::W7(Bit7::new(33).unwrap())
    );
    assert_eq!(w7.core_operation(), None);
    assert_eq!((w7.width(), w7.packed_bits()), (7, 33));
}

#[test]
fn mixed_width_frame_is_not_a_homogeneous_text7_stream() {
    let mut packer = BitPacker::new();
    packer.push(Bit7::new(0x15).unwrap());
    packer.push(sens::Bit3::new(0x05).unwrap());
    let frame = packer.finish();
    assert_eq!(frame.bit_len(), 10);
    assert_eq!(
        Text7::from_packed_w7(&frame).unwrap_err(),
        Text7W7Error::UnalignedBitLen { bit_len: 10 }
    );
}
