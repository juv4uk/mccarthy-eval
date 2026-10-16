/// Визначення — форма з головою def, define або кодом СЕНС 00001001/00001011
/// (так само рахує scripts/generate-functions-md.lisp).
fn defined_names(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = ["(def ", "(define ", "(00001001 ", "(00001011 "]
                .iter()
                .find_map(|head| trimmed.strip_prefix(head))?;
            rest.split_whitespace().next()
        })
        .collect()
}

fn function_reference_section<'a>(reference: &'a str, file: &str) -> &'a str {
    let marker = format!("### {file} (");
    let start = reference
        .find(&marker)
        .unwrap_or_else(|| panic!("FUNCTIONS.md is missing section for {file}"));
    let tail = &reference[start..];
    let end = tail[marker.len()..]
        .find("\n### ")
        .map(|offset| marker.len() + offset)
        .unwrap_or(tail.len());
    &tail[..end]
}

// public_docs_share_current_project_identity_and_extension,
// public_docs_point_to_semantic_authority_instead_of_inventing_one, and
// host_semantic_surface_documentation_tracks_time_ownership were pure
// markdown/doc-text checks (no executable sens behavior involved) and
// were relocated to `cargo xtask verify` per TEST-ARCHITECTURE-1 step 4 —
// see crates/xtask/src/checks.rs. The extension check was also fixed
// there to assert `.lisp` is stated as canonical (not merely mentioned).

#[test]
fn tracked_function_reference_tracks_live_library_definitions() {
    let reference = include_str!("../../../docs/FUNCTIONS.md");

    for (file, source) in [
        ("result-status.lisp", include_str!("../../../lib/result-status.lisp")),
        ("narrate.lisp", include_str!("../../../lib/narrate.lisp")),
        ("translation.lisp", include_str!("../../../lib/translation.lisp")),
        ("quantity.lisp", include_str!("../../../lib/quantity.lisp")),
        ("si.lisp", include_str!("../../../lib/si.lisp")),
    ] {
        let names = defined_names(source);
        let section = function_reference_section(reference, file);
        let expected_heading = format!("### {file} ({})", names.len());
        assert!(
            section.starts_with(&expected_heading),
            "FUNCTIONS.md count for {file} is stale: expected {} definitions",
            names.len()
        );
        for name in names {
            assert!(
                section.contains(&format!("`{name}`")),
                "FUNCTIONS.md section for {file} is missing live definition {name}"
            );
        }
    }
}

// agent_onboarding_records_removed_coordination_surface was a pure
// markdown/doc-text check, relocated to `cargo xtask verify` per
// TEST-ARCHITECTURE-1 step 4 — see crates/xtask/src/checks.rs.
