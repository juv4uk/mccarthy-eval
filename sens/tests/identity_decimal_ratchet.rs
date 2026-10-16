//! Static ratchet for widthless decimal bridges in canonical identity paths.
//! A policy check only: semantic laws remain executable binary SENS programs.

#[derive(Clone, Copy)]
enum Language { Rust, Python }

struct Source<'a> { path: &'static str, language: Language, contents: &'a str }

const SOURCES: &[Source<'static>] = &[
    Source { path: "crates/sens/src/domain_identity.rs", language: Language::Rust, contents: include_str!("../src/domain_identity.rs") },
    Source { path: "crates/sens/src/domain_ladder.rs", language: Language::Rust, contents: include_str!("../src/domain_ladder.rs") },
    Source { path: "crates/sens/src/canonical_reader.rs", language: Language::Rust, contents: include_str!("../src/canonical_reader.rs") },
    Source { path: "crates/sens/src/source_words.rs", language: Language::Rust, contents: include_str!("../src/source_words.rs") },
    Source { path: "crates/sens/src/packed_bits.rs", language: Language::Rust, contents: include_str!("../src/packed_bits.rs") },
    Source { path: "crates/sens/src/frame3_domain.rs", language: Language::Rust, contents: include_str!("../src/frame3_domain.rs") },
    Source { path: "crates/sens/src/universal_frame3.rs", language: Language::Rust, contents: include_str!("../src/universal_frame3.rs") },
    Source { path: "crates/sens/src/binary_framing.rs", language: Language::Rust, contents: include_str!("../src/binary_framing.rs") },
    Source { path: "crates/sens/src/semantic_registry.rs", language: Language::Rust, contents: include_str!("../src/semantic_registry.rs") },
    Source { path: "crates/xtask/src/compiler_export.rs", language: Language::Rust, contents: include_str!("../../xtask/src/compiler_export.rs") },
    Source { path: "scripts/domain_tables.py", language: Language::Python, contents: include_str!("../../../scripts/domain_tables.py") },
    Source { path: "scripts/generate-domain-owner-registry.py", language: Language::Python, contents: include_str!("../../../scripts/generate-domain-owner-registry.py") },
    Source { path: "scripts/generate-domain-width-certificate.py", language: Language::Python, contents: include_str!("../../../scripts/generate-domain-width-certificate.py") },
    Source { path: "scripts/sens_t5_codec.py", language: Language::Python, contents: include_str!("../../../scripts/sens_t5_codec.py") },
    Source { path: "scripts/d7_symbol_codec.py", language: Language::Python, contents: include_str!("../../../scripts/d7_symbol_codec.py") },
    Source { path: "scripts/migrate-domain-ladder-source-batch.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-domain-ladder-source-batch.py") },
    Source { path: "scripts/migrate-exact-domain.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-exact-domain.py") },
    Source { path: "scripts/migrate-selected-lisp.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-selected-lisp.py") },
    Source { path: "scripts/admit-t5-migration.py", language: Language::Python, contents: include_str!("../../../scripts/admit-t5-migration.py") },
    Source { path: "scripts/migrate-t5-batch.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-t5-batch.py") },
    Source { path: "scripts/migrate-approved-t5.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-approved-t5.py") },
    Source { path: "scripts/migrate-three-pass.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-three-pass.py") },
    Source { path: "scripts/migrate-to-sens-codes.py", language: Language::Python, contents: include_str!("../../../scripts/migrate-to-sens-codes.py") },
    Source { path: "scripts/translate-domain-program.py", language: Language::Python, contents: include_str!("../../../scripts/translate-domain-program.py") },
];

fn is_comment(line: &str, language: Language) -> bool {
    let line = line.trim_start();
    match language {
        Language::Rust => line.starts_with("//") || line.starts_with("/*") || line.starts_with('*') || line.starts_with("*/"),
        Language::Python => line.starts_with('#'),
    }
}

fn marker_near(lines: &[&str], index: usize) -> bool {
    let start = index.saturating_sub(2);
    lines[start..=index].iter().any(|line| line.contains("projection-human"))
}

fn has_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn violation(line: &str, language: Language) -> Option<&'static str> {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    match language {
        Language::Rust => {
            if has_any(&compact, &[
                ".parse::<u8>()", ".parse::<u16>()", ".parse::<u32>()",
                ".parse::<u64>()", ".parse::<usize>()", ".parse::<i8>()",
                ".parse::<i16>()", ".parse::<i32>()", ".parse::<i64>()",
            ]) {
                return Some("default decimal .parse::<integer>() in identity path");
            }
            if compact.contains("from_str_radix(")
                && (compact.contains(",10)") || compact.contains(",10usize)"))
            {
                return Some("base-10 from_str_radix in identity path");
            }
            if compact.contains(".to_string()")
                && has_any(&compact, &[
                    "bits.to_string()", "word.to_string()", "payload.to_string()",
                    "packed.to_string()", "identity.to_string()", "coordinate.to_string()",
                    "semantic_id.to_string()", ".packed_bits().to_string()",
                    ".bits().to_string()", "domain_identity.to_string()",
                ])
            {
                return Some("widthless string projection of identity/payload");
            }
            for name in ["bits", "word", "payload", "packed", "identity", "coordinate", "semantic_id", "domain_identity"] {
                if compact.contains(&format!("{{{name}}}")) {
                    return Some("implicit formatting of identity-bearing value");
                }
            }
            for name in ["identity.packed_bits()", "coordinate.bits()", "word.packed_bits()", "semantic_id.packed_byte()"] {
                if compact.contains(&format!("{{}}\",{name}")) {
                    return Some("default decimal formatting of packed identity payload");
                }
            }
            None
        }
        Language::Python => {
            for name in ["bits", "word", "payload", "packed", "identity", "coordinate", "semantic_id"] {
                if compact.contains(&format!("int({name})"))
                    || compact.contains(&format!("str({name})"))
                    || compact.contains(&format!("format({name},\"d\")"))
                    || compact.contains(&format!("format({name},'d')"))
                    || compact.contains(&format!("{{{name}:d}}"))
                {
                    return Some("decimal conversion/formatting of identity-bearing value");
                }
            }
            None
        }
    }
}

fn violations(source: &str, language: Language) -> Vec<String> {
    let production = match language {
        Language::Rust => source.split("#[cfg(test)]").next().unwrap_or(source),
        Language::Python => source,
    };
    let lines: Vec<&str> = production.lines().collect();
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if is_comment(line, language) || marker_near(&lines, index) {
            continue;
        }
        if let Some(reason) = violation(line, language) {
            out.push(format!("line {}: {reason}: {}", index + 1, line.trim()));
        }
    }
    out
}

#[test]
fn canonical_identity_paths_have_no_widthless_decimal_bridge() {
    let mut failures = Vec::new();
    for source in SOURCES {
        for problem in violations(source.contents, source.language) {
            failures.push(format!("{}: {problem}", source.path));
        }
    }
    assert!(
        failures.is_empty(),
        "DECIMAL-IDENTITY-RATCHET: BLOCKED\n{}",
        failures.join("\n")
    );
}

#[test]
fn policy_rejects_decimal_bridges_but_keeps_explicit_binary_projection() {
    assert!(!violations("let raw = bits.parse::<u8>().unwrap();", Language::Rust).is_empty());
    assert!(!violations("let raw = u8::from_str_radix(bits, 10).unwrap();", Language::Rust).is_empty());
    assert!(!violations("let text = identity.to_string();", Language::Rust).is_empty());
    assert!(!violations("let text = format!(\"{}\", identity.packed_bits());", Language::Rust).is_empty());
    assert!(violations("let raw = u8::from_str_radix(bits, 2).unwrap();", Language::Rust).is_empty());
    assert!(violations("let text = format!(\"{:08b}\", identity.packed_bits());", Language::Rust).is_empty());
    assert!(violations("// projection-human: display only\nlet text = identity.to_string();", Language::Rust).is_empty());
    assert!(!violations("packed = int(bits)", Language::Python).is_empty());
    assert!(!violations("rendered = format(payload, \"d\")", Language::Python).is_empty());
    assert!(violations("packed = int(bits, 2)", Language::Python).is_empty());
    assert!(violations("# projection-human: explicit human projection\nrendered = str(identity)", Language::Python).is_empty());
}
