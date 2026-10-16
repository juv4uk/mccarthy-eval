//! Presentation from the current 128/128 D7 v2 keyboard map only.
//! No phonetic D7 v1 residents or reserved-cell exceptions.
use sens::{
    Bit7, BinarySourceWord, DomainIdentity, PresentationLanguage, Value,
    render_value_for_presentation,
};

#[test]
fn all_128_w7_cells_preserve_identity_and_project_from_the_owner_keyboard() {
    for raw in 0u8..=127 {
        let word = BinarySourceWord::W7(Bit7::new(raw).unwrap());
        let identity = DomainIdentity::from_source_word(word);
        assert_eq!(identity.width(), 7);
        assert_eq!(identity.packed_bits(), u16::from(raw));
        assert_eq!(identity.source_word(), word);
        let value = Value::DomainIdentity(identity);
        let exact = format!("{raw:07b}");
        assert_eq!(value.to_string(), exact);
        assert_eq!(render_value_for_presentation(&value, PresentationLanguage::Canonical), exact);
        assert_eq!(render_value_for_presentation(&value, PresentationLanguage::English), exact);
        let uk = sens::d7_keyboard::project(raw, sens::d7_keyboard::D7Mode::Uk).unwrap();
        let san = sens::d7_keyboard::project(raw, sens::d7_keyboard::D7Mode::San).unwrap();
        assert_eq!(render_value_for_presentation(&value, PresentationLanguage::Ukrainian),
                   uk.to_string());
        assert_eq!(render_value_for_presentation(&value, PresentationLanguage::Sanskrit),
                   san.to_string());
    }
}

#[test]
fn identical_payload_across_d7_and_d8_stays_different() {
    let d7 = DomainIdentity::from_source_word(BinarySourceWord::W7(Bit7::new(1).unwrap()));
    let d8 = DomainIdentity::from_source_word(BinarySourceWord::W8(sens::Bit8::new(1).unwrap()));
    assert_ne!(d7, d8);
    assert_eq!((d7.width(), d7.packed_bits()), (7, 1));
    assert_eq!((d8.width(), d8.packed_bits()), (8, 1));
}
