//! Physical T5 round-trip canary for exact-width binary source.
//! QUOTE's result law is owned by the SENS/Lisp witness, not Rust.
use sens::{decode_ternary_program, encode_binary_projection_ternary, open_ternary_program, render_ternary_words_spaced};

const T5: &[u8] =
    include_bytes!("../../../tests/fixtures/migration-quote-cohort-main/quote-legacy.sens");

#[test]
fn migrated_quote_fixture_preserves_exact_physical_t5() {
    assert_eq!(T5, [0x63, 0x89, 0x06, 0xa1]);

    let words = decode_ternary_program(T5).expect("canonical T5 bytes");
    let visible = render_ternary_words_spaced(&words);
    assert_eq!(visible, "10 001 00 000 01");
    assert_eq!(open_ternary_program(T5).unwrap(), visible);
    assert_eq!(encode_binary_projection_ternary(&visible).unwrap(), T5);

}
