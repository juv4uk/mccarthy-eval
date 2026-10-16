//! Frame3 research: actual Rust D2 reader vs independent recursive grammar.
//!
//! This is admission parity, not executable validity and not a production
//! compression profile. The physical Frame3 owner keeps sole codec authority.
use sens::parse_canonical_binary;

fn independent_accepts(words: &[String]) -> bool {
    fn skip_separators(words: &[String], pos: &mut usize) {
        while words.get(*pos).is_some_and(|s| s == "00") {
            *pos += 1;
        }
    }

    fn expr(words: &[String], pos: &mut usize) -> bool {
        match words.get(*pos).map(String::as_str) {
            Some("10") => list(words, pos),
            Some("00" | "01" | "11") | None => false,
            Some(_) => {
                *pos += 1;
                true
            }
        }
    }

    fn list(words: &[String], pos: &mut usize) -> bool {
        if words.get(*pos).map(String::as_str) != Some("10") {
            return false;
        }
        *pos += 1;
        let mut heads = 0usize;
        loop {
            skip_separators(words, pos);
            match words.get(*pos).map(String::as_str) {
                Some("01") => {
                    *pos += 1;
                    return true;
                }
                Some("11") => {
                    if heads == 0 {
                        return false;
                    }
                    *pos += 1;
                    skip_separators(words, pos);
                    if !expr(words, pos) {
                        return false;
                    }
                    skip_separators(words, pos);
                    if words.get(*pos).map(String::as_str) != Some("01") {
                        return false;
                    }
                    *pos += 1;
                    return true;
                }
                Some(_) => {
                    if !expr(words, pos) {
                        return false;
                    }
                    heads += 1;
                }
                None => return false,
            }
        }
    }

    let mut pos = 0usize;
    list(words, &mut pos) && pos == words.len()
}

fn enumerate_inner(
    budget: usize,
    words: &mut Vec<String>,
    on_complete: &mut impl FnMut(&[String]),
) {
    if budget == 0 {
        on_complete(words);
        return;
    }
    for width in 1..=9.min(budget) {
        for value in 0usize..(1usize << width) {
            words.push(format!("{value:0width$b}"));
            enumerate_inner(budget - width, words, on_complete);
            words.pop();
        }
    }
}

#[test]
fn exact_d2_reader_matches_independent_grammar_on_every_short_typed_word_stream() {
    // Independent Python enumerator confirms these admitted inner counts.
    // Counts include redundant D2 separators in exact typed input.
    const COUNTS: [usize; 9] = [1, 2, 5, 20, 82, 292, 1092, 3996, 14793];
    for (bits, expected) in COUNTS.into_iter().enumerate() {
        let mut admitted = 0usize;
        let mut seen = 0usize;
        enumerate_inner(bits, &mut Vec::new(), &mut |body| {
            let mut words = Vec::with_capacity(body.len() + 2);
            words.push("10".to_owned());
            words.extend(body.iter().cloned());
            words.push("01".to_owned());
            let expected_accept = independent_accepts(&words);
            let rust_accept = parse_canonical_binary(&words.join(" "))
                .is_ok_and(|forms| forms.len() == 1);
            assert_eq!(
                rust_accept,
                expected_accept,
                "D2 reader parity on inner bits={bits} exact words={words:?}"
            );
            seen += 1;
            admitted += usize::from(rust_accept);
        });
        assert_eq!(admitted, expected, "D2 exact grammar count for {bits} bits");
        assert!(seen >= admitted, "not all valid sequences were visited");
    }
}

#[test]
fn d2_token_roles_dot_tail_and_reserved_widths_are_not_silently_coerced() {
    for accepted in [
        "10 01",
        "10 00 01",
        "10 00 00 01",
        "10 0 11 1 01",
        "10 0 11 00 1 01",
        "10 0 11 10 1 01 01",
        "10 10 010 01 01",
        "10 001 10 10 010 01 01 01", // quoted ((ATOM)) as data
    ] {
        assert!(parse_canonical_binary(accepted).is_ok(), "{accepted}");
    }
    for rejected in [
        "10 11 01",            // dot without a head
        "10 01 01",            // extra close
        "10 0 11 01",          // dot missing its tail
        "10 0 11 1 0 01",      // two dotted tails
        "10 10 01",            // nested close missing
        "10 0 11 1",           // truncated
        "10 0000000000 01",   // unframed W10 is out of scope
    ] {
        assert!(parse_canonical_binary(rejected).is_err(), "{rejected}");
    }
}

#[test]
fn exact_width_d1_and_d3_through_d9_are_not_conflated() {
    for width in [1usize, 3, 4, 5, 6, 7, 8, 9] {
        for value in 0usize..(1usize << width) {
            // Admission of unoccupied domain slots is separately law-owned.
            let source = format!("10 {value:0width$b} 01");
            assert!(
                parse_canonical_binary(&source).is_ok(),
                "structural D2 grammar must retain typed width: {source}"
            );
        }
    }
}
