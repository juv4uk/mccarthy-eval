use sens::{eval_program, load_core_library, lower_program, parse, ExprKind, Session};
use std::fs;
use std::path::PathBuf;

#[path = "support/x86_64_block_decoder.rs"]
mod x86_64_block_decoder;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_lisp_file(path: &str, session: &mut Session) {
    let path = repo_root().join(path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must exist: {error}", path.display()));
    eval_program(&source, session)
        .unwrap_or_else(|error| panic!("{} must load as ordinary sens: {error}", path.display()));
}

fn machine_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core");
    load_lisp_file("lib/machine/encoding/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/admission/x86-64.lisp", &mut session);
    load_lisp_file("lib/machine/lowering/semantic-x86-64.lisp", &mut session);
    session
}

fn eval_value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .value
        .to_string()
}

fn parse_bytes(rendered: &str) -> Vec<u8> {
    rendered
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.parse::<u8>().expect("machine byte"))
        .collect()
}

#[test]
fn current_eq_and_cond_surfaces_lower_to_exact_d3_identities() {
    let eq = lower_program(&parse("(тотожне? 2 3)").expect("EQ source"));
    let ExprKind::DomainCall(eq_identity, _) = eq[0].kind else {
        panic!("current EQ surface must lower to DomainCall");
    };
    assert_eq!((eq_identity.width(), eq_identity.packed_bits()), (3, 0b101));

    let cond_source =
        "(за-умовою ((тотожне? 2 3) #d111) (1 #d222))";
    let cond = lower_program(&parse(cond_source).expect("COND source"));
    let ExprKind::DomainCall(cond_identity, clauses) = &cond[0].kind else {
        panic!("current COND surface must lower to DomainCall");
    };
    assert_eq!((cond_identity.width(), cond_identity.packed_bits()), (3, 0b110));

    let ExprKind::List(first_clause) = &clauses[0].kind else {
        panic!("COND first clause must remain structured");
    };
    assert!(matches!(
        &first_clause[0].kind,
        ExprKind::DomainCall(identity, _)
            if identity.width() == 3 && identity.packed_bits() == 0b101
    ));
}

#[test]
fn exact_d3_eq_cond_pair_selects_existing_bounded_x86_composition() {
    let mut session = machine_session();

    let current = eval_value(
        "(x86-lower-current-eq-cond-u64-forms 3 5 3 6 2 3 111 222)",
        &mut session,
    );
    let existing = eval_value(
        "(x86-lower-eq-cond-u64-forms 2 3 111 222)",
        &mut session,
    );
    assert_eq!(current, existing);

    let encoded = eval_value(
        "(x86-encode-current-eq-cond-u64 3 5 3 6 2 3 111 222)",
        &mut session,
    );
    assert!(
        encoded.starts_with('(') && !encoded.contains("rejected"),
        "exact D3 EQ+COND must reach admitted bytes, got {encoded}"
    );

    let bytes = parse_bytes(&encoded);
    let decoded = x86_64_block_decoder::decode_machine_block(&bytes)
        .unwrap_or_else(|error| panic!("independent decoder rejected EQ+COND bytes: {error}"));
    assert_eq!(format!("({})", decoded.join(" ")), current);
}

#[test]
fn eq_cond_composition_fails_closed_on_wrong_exact_identity() {
    let mut session = machine_session();

    for source in [
        "(x86-encode-current-eq-cond-u64 3 4 3 6 2 3 111 222)",
        "(x86-encode-current-eq-cond-u64 3 5 3 7 2 3 111 222)",
    ] {
        assert_eq!(
            eval_value(source, &mut session),
            "unsupported-current-domain-eq-cond-u64",
            "{source}"
        );
    }
}

#[test]
fn current_eq_cond_route_does_not_publish_numeric_boolean_as_language_result() {
    let source = fs::read_to_string(
        repo_root().join("lib/machine/lowering/semantic-x86-64.lisp"),
    )
    .expect("semantic x86 lowering");

    let start = source
        .find("(00001001 x86-lower-current-eq-cond-u64-forms")
        .expect("current EQ+COND route");
    let block = &source[start..];
    assert!(block.contains("x86-lower-eq-cond-u64-forms"));
    assert!(
        !block.contains("numeric-boolean"),
        "machine route must not mint a numeric boolean language contract"
    );
}
