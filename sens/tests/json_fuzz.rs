//! Deterministic fuzz-style robustness checks for the boundary code most
//! likely to hide edge cases: the hand-rolled JSON decoder backing
//! `sens::parse_json`. Property: for ANY input — valid, truncated,
//! random bytes, deep nesting, huge escapes — parsing must return Ok or
//! Err. It must never panic and never hang.

use sens::parse_json;

/// xorshift64* - tiny deterministic PRNG so failures are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }
}

fn must_not_panic(input: &str) {
    // The only contract: returns instead of panicking/hanging.
    let _ = parse_json(input);
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(0xDEADBEEF);
    for _ in 0..2000 {
        let len = (rng.next() % 64) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        match String::from_utf8(bytes.clone()) {
            Ok(text) => must_not_panic(&text),
            Err(e) => {
                // Invalid UTF-8 cannot even enter the str-typed API; the
                // lossy form still exercises the parser with odd bytes.
                must_not_panic(&String::from_utf8_lossy(&e.into_bytes()));
            }
        }
    }
}

#[test]
fn structured_garbage_never_panic() {
    let fragments = [
        "{",
        "}",
        "[",
        "]",
        "\"",
        "\\",
        ":",
        ",",
        "{\"a\"",
        "{\"a\":}",
        "[1,",
        "[[",
        "[[]]",
        "\"\\u",
        "\"\\u12",
        "\"\\uD800",
        "\"\\uD800\\u",
        "{\"a\":[true,false,null,-0.5e+10]}",
        "123456789012345678901234567890",
        "-99999999999999999999999e-99999",
        "{\"k\":\"\\uDC00 lone\"}",
        "\t\n\r ",
        "\"unterminated",
        "{\"a\":1}trailing",
        "[1 2]",
        "{\"a\" 1}",
        "tru",
        "nul",
    ];
    for fragment in fragments {
        must_not_panic(fragment);
    }
    // Every prefix of a valid document (truncation at every byte).
    let doc = r#"{"model":"qwen3:4b","messages":[{"role":"user","content":"say \"hi\"\n\u0433"}],"tools":[],"n":-1.5e3}"#;
    for i in 0..doc.len() {
        must_not_panic(&doc[..i]);
    }
}

#[test]
fn deep_nesting_is_rejected_not_crashed() {
    // 10k nested arrays: either parses (stack permitting) or errors named;
    // the historical failure mode would be a Rust stack overflow abort,
    // which is exactly what this test exists to catch if limits regress.
    let deep = "[".repeat(10_000) + &"]".repeat(10_000);
    let _ = parse_json(&deep);
}
