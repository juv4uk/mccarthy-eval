use std::fs;
use std::path::Path;

fn repository_file(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("sens crate should live under repository/crates");
    fs::read_to_string(root.join(name)).unwrap()
}

fn numeric_field(source: &str, name: &str) -> u64 {
    let marker = format!("({name} . ");
    let start = source
        .find(&marker)
        .unwrap_or_else(|| panic!("missing contract field: {name}"))
        + marker.len();
    let end = source[start..]
        .find(')')
        .map(|offset| start + offset)
        .expect("contract field should close");
    source[start..end]
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("contract field should be numeric: {name}"))
}

#[test]
fn typed_buffer_record_matches_ratified_language_contract() {
    let proposal = repository_file("typed-buffer-proposal.lisp");
    let expressions = sens::parse(&proposal).expect("proposal must remain valid sens data");
    assert_eq!(expressions.len(), 1);
    assert!(proposal.contains("(status . ratified-implemented)"));
    assert!(proposal.contains("(current-language-contract . (2 2))"));
    assert!(proposal.contains("(implicit-exact-to-f32 . forbidden)"));

    // The proposal records the contract at which typed buffers were ratified,
    // not the newest unrelated language-contract revision. Later contract
    // bumps must not force this historical ratification record to lie about
    // its origin; they only must not regress below the 2.2 boundary.
    let language_contract = repository_file("language-contract.lisp");
    let current = (
        numeric_field(&language_contract, "major"),
        numeric_field(&language_contract, "minor"),
    );
    assert!(
        current >= (2, 2),
        "current language contract {current:?} predates typed-buffer ratification 2.2"
    );
}
