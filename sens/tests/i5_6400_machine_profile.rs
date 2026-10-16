use std::collections::HashSet;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
}

#[test]
fn legacy_i5_6400_profile_is_explicit_sid8_compatibility_only() {
    let root = repo_root();
    let profile_path = root.join("lib/machine/intel-core-i5-6400.lisp");
    let profile = std::fs::read_to_string(&profile_path)
        .unwrap_or_else(|error| panic!("missing i5-6400 compatibility profile at {profile_path:?}: {error}"));

    sens::parse(&profile).expect("legacy i5-6400 profile must remain valid sens data");

    assert!(profile.contains("Legacy Intel Core i5-6400 / Skylake SID8 compatibility projection."));
    assert!(profile.contains("NOT current semantic authority"));
    assert!(profile.contains("lib/machine/profile/current-domain-x86-64.lisp"));

    for (sid, expected_machine_path) in [
        ("00000011", "CMP/SETE"),
        ("00000101", "LOAD-pair-head"),
        ("00000110", "LOAD-pair-tail"),
        ("00001100", "ADD"),
        ("00001101", "SUB/NEG"),
        ("00001110", "IMUL"),
        ("00011010", "CMP/SETL"),
        ("00011011", "CMP/SETG"),
    ] {
        let row_prefix = format!("({sid} ");
        let row = profile
            .lines()
            .map(str::trim_start)
            .find(|line| line.starts_with(&row_prefix))
            .unwrap_or_else(|| panic!("legacy i5-6400 profile missing compatibility SID {sid}"));
        assert!(
            row.contains(expected_machine_path),
            "compatibility SID {sid} must preserve {expected_machine_path:?}; row: {row}"
        );
    }
}

#[test]
fn legacy_i5_6400_rows_are_unique_8bit_compatibility_keys() {
    let profile = include_str!("../../../lib/machine/intel-core-i5-6400.lisp");
    let mut seen = HashSet::new();
    let mut projected = 0usize;

    for line in profile.lines().map(str::trim_start) {
        let Some(rest) = line.strip_prefix('(') else {
            continue;
        };
        let Some((id, after_id)) = rest.split_once(' ') else {
            continue;
        };
        if id.len() != 8 || !id.bytes().all(|byte| matches!(byte, b'0' | b'1')) {
            continue;
        }

        assert!(!after_id.is_empty(), "legacy machine row must continue after SID");
        projected += 1;
        assert!(
            seen.insert(id),
            "legacy i5-6400 compatibility projection must not duplicate SID {id}"
        );
    }

    assert!(
        projected >= 50,
        "legacy compatibility snapshot should preserve a meaningful historical subset"
    );
}

#[test]
fn current_i5_6400_profile_uses_width_safe_domain_keys_for_migrated_cpu_slice() {
    let current = include_str!("../../../lib/machine/profile/current-domain-x86-64.lisp");

    assert!(current.contains("(machine-domain-profile/2"));
    assert!(current.contains("(cpu intel-core-i5-6400)"));
    assert!(current.contains("(key-shape width+packed-bits)"));

    for (width, packed_bits, expected_machine_path) in [
        (5, 8, "CMP+SETE+MOVZX"), // D5 ZEROP
        (5, 10, "ADD"),       // D5 PLUS
        (5, 11, "SUB"),       // D5 DIFFERENCE
        (5, 22, "IMUL"),      // D5 TIMES
        (5, 23, "CQO+IDIV"),  // D5 QUOTIENT
        (5, 26, "CMP+SETL"),  // D5 LESSP
        (5, 27, "CMP+SETG"),  // D5 GREATERP
        (6, 14, "ADD"),        // D6 ADD1
        (6, 15, "SUB"),        // D6 SUB1
        (3, 5, "CMP/SETE"),   // D3 EQ
        (3, 6, "CMP+Jcc"),    // D3 COND
        (3, 7, "STORE-pair"), // D3 CONS
        (3, 4, "LOAD-pair-head"), // D3 CAR
        (3, 3, "LOAD-pair-tail"), // D3 CDR
    ] {
        let row_prefix = format!("({width} {packed_bits} ");
        let row = current
            .lines()
            .map(str::trim_start)
            .find(|line| line.starts_with(&row_prefix))
            .unwrap_or_else(|| {
                panic!("current profile missing width-safe key ({width},{packed_bits})")
            });
        assert!(
            row.contains(expected_machine_path),
            "width-safe key ({width},{packed_bits}) must advertise {expected_machine_path:?}; row: {row}"
        );
    }

    // Same packed payload under D4 must never inherit current D5 rows.
    for packed_bits in [8, 10, 11] {
        let row_prefix = format!("(4 {packed_bits} ");
        assert!(
            !current.lines().map(str::trim_start).any(|line| line.starts_with(&row_prefix)),
            "D4 packed payload {packed_bits} must not inherit a D5 machine row"
        );
    }

    // D5 selectors with the same packed payload must not inherit D6 ADD1/SUB1.
    for packed_bits in [14, 15] {
        let row_prefix = format!("(5 {packed_bits} ");
        assert!(
            !current.lines().map(str::trim_start).any(|line| line.starts_with(&row_prefix)),
            "D5 packed payload {packed_bits} must not inherit a D6 machine row"
        );
    }
}

#[test]
fn generated_function_table_keeps_legacy_i5_projection_out_of_semantic_table() {
    let markdown = include_str!("../../../docs/generated/function-table.md");
    assert!(
        markdown.contains("Intel Core i5-6400 / Skylake"),
        "historical human function table may preserve the processor compatibility column"
    );

    let semantic_table = include_str!("../../../lib/generated/function-table.lisp");
    assert!(
        !semantic_table.contains("intel-core-i5-6400"),
        "processor-specific realization must not contaminate semantic machine-readable data"
    );
}

#[test]
fn function_table_generator_with_machine_projection_is_valid_lisp() {
    let source = include_str!("../../../scripts/generate-function-table.lisp");
    sens::parse(source).expect("function-table generator must remain valid sens source");
}
