//! Bounded Rust rank/unrank proof versus the real D2 reader.
//! No live Frame3 physical file-mode changes are authorized by this test.

use std::collections::HashSet;

use sens::frame3_grammar::{
    count_inner, decode_body_at_known_length, encode_body_at_known_length, rank_single_root,
    unrank_binary_source_words, unrank_single_root, rank_binary_source_words,
    ExactWord, GrammarRankError, MAX_INNER_BITS,
};
use sens::{parse_binary_source_words, parse_canonical_binary};
use sens::frame3_semantic::{check_executable_code, AdmissionWitness};

fn word(bits: &str) -> ExactWord {
    ExactWord::new(
        bits.len() as u8,
        u16::from_str_radix(bits, 2).unwrap(),
    )
    .unwrap()
}

fn words(source: &str) -> Vec<ExactWord> {
    source.split_whitespace().map(word).collect()
}

fn checked(source: &str) {
    let original = words(source);
    let numbered = rank_single_root(&original).unwrap();
    assert_eq!(
        unrank_single_root(numbered.inner_bits as usize, numbered.ordinal).unwrap(),
        original
    );
    let (bits, bytes) = encode_body_at_known_length(&original).unwrap();
    assert_eq!(bits, numbered.inner_bits as usize);
    assert_eq!(decode_body_at_known_length(bits, &bytes).unwrap(), original);
    assert_eq!(parse_canonical_binary(source).unwrap().len(), 1);
}

fn enumerate_exact_words(
    bits: usize,
    current: &mut Vec<String>,
    done: &mut impl FnMut(&[String]),
) {
    if bits == 0 {
        done(current);
        return;
    }
    for width in 1..=bits.min(9) {
        for value in 0..(1usize << width) {
            current.push(format!("{value:0width$b}"));
            enumerate_exact_words(bits - width, current, done);
            current.pop();
        }
    }
}

#[test]
fn python_dp_count_vectors_and_exact_width_ordinals_match() {
    const PYTHON_COUNTS: [u128; 9] = [1, 2, 5, 20, 82, 292, 1092, 3996, 14793];
    for (bits, count) in PYTHON_COUNTS.into_iter().enumerate() {
        assert_eq!(count_inner(bits).unwrap(), count);
    }
    assert_eq!(rank_single_root(&words("10 01")).unwrap().ordinal, 0);
    assert_eq!(rank_single_root(&words("10 0 01")).unwrap().ordinal, 0);
    assert_eq!(rank_single_root(&words("10 1 01")).unwrap().ordinal, 1);
    assert_eq!(rank_single_root(&words("10 00 01")).unwrap().ordinal, 4);
    assert_eq!(unrank_single_root(2, 4).unwrap(), words("10 00 01"));
}

#[test]
fn exact_d2_rank_matches_real_rust_reader_for_every_short_source() {
    const COUNTS: [usize; 9] = [1, 2, 5, 20, 82, 292, 1092, 3996, 14793];
    for (bits, expected_count) in COUNTS.into_iter().enumerate() {
        let mut count = 0usize;
        let mut ranks = HashSet::<u128>::new();
        enumerate_exact_words(bits, &mut Vec::new(), &mut |body| {
            let mut all = vec!["10".to_string()];
            all.extend(body.iter().cloned());
            all.push("01".to_string());
            let source = all.join(" ");
            let rust_d2 = parse_canonical_binary(&source).is_ok_and(|ast| ast.len() == 1);
            let exact_words = all.iter().map(|s| word(s)).collect::<Vec<_>>();
            let ranked = rank_single_root(&exact_words);
            assert_eq!(
                ranked.is_ok(),
                rust_d2,
                "reader/grammar disagreement: {source}"
            );
            if let Ok(rank) = ranked {
                count += 1;
                assert!(ranks.insert(rank.ordinal), "duplicate rank: {source}");
                assert_eq!(rank.inner_bits as usize, bits);
                assert_eq!(
                    unrank_single_root(bits, rank.ordinal).unwrap(),
                    exact_words
                );
            }
        });
        assert_eq!(count, expected_count, "bit length {bits}");
        assert_eq!(ranks.len(), expected_count);
        for i in 0..count {
            assert!(ranks.contains(&(i as u128)), "missing rank {i} for {bits} bits");
        }
    }
}

#[test]
fn nested_dot_and_quoted_nonexecutable_structure_stay_exact() {
    for source in [
        "10 01",
        "10 00 01",
        "10 00 00 01",
        "10 0 11 1 01",
        "10 0 11 00 1 01",
        "10 0 11 10 1 01 01",
        "10 10 010 01 01", // D2 syntax only: ((ATOM)) is not an executable SENS call.
        "10 001 10 10 010 01 01 01", // (QUOTE ((ATOM))) retains original data.
    ] {
        checked(source);
    }
    // Four different exact word widths share no accidental alias.
    for width in [1, 3, 4, 5, 6, 7, 8, 9] {
        for value in 0..(1usize << width) {
            let source = format!("10 {value:0width$b} 01");
            checked(&source);
        }
    }
}

#[test]
fn fail_closed_on_unknown_lengths_ambiguous_roots_and_unused_codes() {
    assert_eq!(count_inner(MAX_INNER_BITS + 1), Err(GrammarRankError::TooLarge));
    assert_eq!(ExactWord::new(2, 4), Err(GrammarRankError::InvalidWord));
    assert_eq!(ExactWord::new(10, 0), Err(GrammarRankError::InvalidWord));

    for source in [
        "10",
        "10 01 10 01",
        "10 11 01",
        "10 0 11 01",
        "10 0 11 1 0 01",
        "10 0 11 1",
        "10 10 01",
        "10 0 01 01",
    ] {
        assert!(rank_single_root(&words(source)).is_err(), "{source}");
    }
    assert_eq!(unrank_single_root(2, 5), Err(GrammarRankError::OutOfRange));
    assert_eq!(
        decode_body_at_known_length(2, &[255]),
        Err(GrammarRankError::OutOfRange)
    );
    assert_eq!(
        decode_body_at_known_length(2, &[0, 0]),
        Err(GrammarRankError::NonCanonical)
    );
    assert_eq!(encode_body_at_known_length(&words("10 01")).unwrap(), (0, vec![]));
    assert_eq!(decode_body_at_known_length(0, &[]).unwrap(), words("10 01"));
    assert_eq!(decode_body_at_known_length(0, &[0]), Err(GrammarRankError::NonCanonical));
}

#[test]
fn codec_typed_word_bridge_preserves_every_domain_and_d2_marker() {
    for width in [1usize, 2, 3, 4, 5, 6, 7, 8, 9] {
        for value in 0..(1usize << width) {
            let source = format!("10 {value:0width$b} 01");
            // W2 01 and 10 are NOT single-root list interiors; 11 is
            // invalid before any list head. W2 00 is a valid separator.
            let parsed = parse_canonical_binary(&source);
            if !parsed.as_ref().is_ok_and(|ast| ast.len() == 1) {
                continue;
            }
            let original = parse_binary_source_words(&source)
                .unwrap()
                .into_iter()
                .map(|token| token.word)
                .collect::<Vec<_>>();
            let number = rank_binary_source_words(&original).unwrap();
            let restored = unrank_binary_source_words(
                number.inner_bits as usize,
                number.ordinal,
            )
            .unwrap();
            assert_eq!(restored, original, "{source}");
        }
    }
    for source in [
        "10 0 11 00 1 01",
        "10 001 10 10 010 01 01 01",
        "10 00 00 01",
    ] {
        let original = parse_binary_source_words(source)
            .unwrap()
            .into_iter()
            .map(|token| token.word)
            .collect::<Vec<_>>();
        let ranked = rank_binary_source_words(&original).unwrap();
        assert_eq!(
            unrank_binary_source_words(ranked.inner_bits as usize, ranked.ordinal).unwrap(),
            original,
            "{source}"
        );
    }
}

#[test]
fn code_data_arity_redundancy_witness_is_contextual_and_fail_conservative() {
    fn verdict(s: &str) -> AdmissionWitness {
        let expressions = parse_canonical_binary(s).unwrap();
        assert_eq!(expressions.len(), 1);
        check_executable_code(&expressions[0])
    }
    let dead = AdmissionWitness::ProvablyImpossible;
    let unknown = AdmissionWitness::NotProvenImpossible;

    assert_eq!(verdict("10 010 01"), dead);  // ATOM has no argument.
    assert_eq!(verdict("10 10 010 01 01"), dead); // ((ATOM))
    assert_eq!(verdict("10 001 01"), dead);  // QUOTE has no argument.
    assert_eq!(verdict("10 001 0 1 01"), dead); // QUOTE has two arguments.
    assert_eq!(verdict("10 101 0 01"), dead); // EQ needs two arguments.
    assert_eq!(verdict("10 111 0 01"), dead); // CONS needs two arguments.
    assert_eq!(verdict("10 010 10 010 01 01"), dead); // invalid evaluated operand.
    assert_eq!(
        verdict("10 001 10 10 010 01 01 01"), unknown,
        "QUOTE preserves invalid-to-execute nested syntax as data"
    );
    assert_eq!(verdict("10 010 0 01"), unknown);  // Arity alone is not full validity.
    assert_eq!(verdict("10 110 10 010 01 01"), unknown); // COND owns clauses.
    assert_eq!(verdict("10 000 01"), unknown); // Empty D3 is NOT a callable identity.
    assert_eq!(verdict("10 01"), unknown); // () is a value.
    assert_eq!(verdict("10 10 001 0 01 01"), unknown); // Dynamic outer head.
}
