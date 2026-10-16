//! Repository-wide invariant for every FASL consumer, including WASM.
//! A committed core4.lisp.fasl must be bound to the exact current lib/core4.lisp
//! source. The native CLI already falls back when this is false; consumers
//! that embed only the snapshot must never be allowed to build from a green
//! repository with a stale snapshot in the first place.

#[test]
fn committed_core_fasl_matches_current_core_source() {
    let fasl = include_bytes!("../../../lib/core4.lisp.fasl");
    let (snapshot_expressions, embedded_hash) = sens::fasl_decode_program(fasl)
        .expect("committed lib/core4.lisp.fasl must decode");
    let current_source = sens::CORE_LIBRARY_SOURCE;
    let current_hash = sens::sha256_source(current_source.as_bytes());

    assert_eq!(
        embedded_hash, current_hash,
        "lib/core4.lisp.fasl is stale; regenerate it from the current lib/core4.lisp"
    );

    let current_expressions =
        sens::parse_mixed_exact_domain_core_source("lib/core4.lisp", current_source)
            .expect("Core4 source must parse through the path-bound exact-domain reader");

    // FASL intentionally drops source spans. Round-trip the expected AST through
    // the same format so the comparison checks semantic nodes/domains, not
    // transport-only byte offsets.
    let expected_fasl = sens::fasl_encode(&current_expressions, &current_hash);
    let (expected_snapshot_expressions, expected_hash) =
        sens::fasl_decode_program(&expected_fasl)
            .expect("fresh path-bound Core4 AST must encode/decode as FASL");
    assert_eq!(expected_hash, current_hash);
    assert_eq!(
        snapshot_expressions, expected_snapshot_expressions,
        "Core4 FASL must preserve exact-domain parser output, not an ordinary-parse AST; regenerate with gen-fasl"
    );
}


/// #5408: порожня, одна й дві послідовні прив'язки мають працювати з
/// поточного Lisp-макросу, незалежно від людської поверхні імен.
#[test]
fn poslidovni_pryviazky_d6_vykonuiutsia_i_ne_prykhovuiut_nevidome_imia() {
    for source in [
        "(let* () 7)",
        "(let* ((x 7)) x)",
        "(let* ((x 7) (y x)) y)",
        "(нехай* ((основа 7) (наслідок основа)) наслідок)",
        "(let* ((основа 7) (наслідок основа)) наслідок)",
        "(нехай* ((x 7) (y x)) y)",
    ] {
        let mut session = sens::Session::default();
        sens::load_core_library(&mut session)
            .expect("чинний Core4 мусить завантажитися");
        let expressions = sens::parse(source).expect("допущена форма D6");
        let outcome = sens::eval_parsed_expressions(&expressions, &mut session)
            .unwrap_or_else(|err| panic!("нехай* не створило лексичну рамку для {source}: {err}"));
        assert_eq!(outcome.value.to_string(), "7", "неправильне значення для {source}");
    }

    let mut session = sens::Session::default();
    sens::load_core_library(&mut session).expect("поточний Core4");
    let unknown = sens::parse("(нехай* ((основа 7)) наслідок)")
        .expect("допущений негативний свідок");
    assert!(
        sens::eval_parsed_expressions(&unknown, &mut session).is_err(),
        "невідомий символ повинен завершуватися помилкою, а не прихованим допуском"
    );
}


#[test]
fn core4_length_terminates_on_exact_d4_null_not_on_nonempty_atom_negation() {
    // #5408: Observe the Lisp-owned source law through the real Core4
    // bootstrap. A prior NOT(ATOM(values)) guard returned zero for every
    // nonempty proper list, breaking the seven-field SI constant witness.
    let mut session = sens::Session::default();
    sens::load_core_library(&mut session).expect("exact current Core4 must load");
    for (source, expected) in [
        ("(length (001 ()))", "0"),
        ("(length (001 (7)))", "1"),
        ("(length (001 (7 6 5 4 3 2 1)))", "7"),
    ] {
        let expressions = sens::parse(source).expect("admitted exact D3 QUOTE fixture");
        let outcome = sens::eval_parsed_expressions(&expressions, &mut session)
            .unwrap_or_else(|err| panic!("Lisp-owned length failed for {source}: {err}"));
        assert_eq!(
            outcome.value.to_string(),
            expected,
            "D4 NULL must distinguish empty from nonempty proper lists: {source}"
        );
    }
}
