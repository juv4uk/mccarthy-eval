//! #2766 — mechanical D5 exact-width source boundary witness.
//!
//! This proves only representation/source-boundary support for all 32 exact
//! five-bit words. It does not claim semantic residency, registry admission,
//! lowering or evaluator dispatch.

use sens::{parse_binary_source_words, BinarySourceWord};

#[test]
fn every_exact_d5_word_round_trips_as_w5_without_padding() {
    for raw in 0u8..32 {
        let source = format!("{raw:05b}");
        let tokens = parse_binary_source_words(&source).expect("exact D5 source word");
        assert_eq!(tokens.len(), 1);

        let word = tokens[0].word;
        assert_eq!(word.width(), 5);
        assert_eq!(word.packed_bits(), u16::from(raw));
        assert_eq!(word.to_string(), source);
        assert!(matches!(word, BinarySourceWord::W5(_)));
    }
}
