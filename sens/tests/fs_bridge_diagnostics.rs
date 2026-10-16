//! Path-pinned witness for Lisp-owned filesystem adapter semantics.
//! This narrows full store failures to package lookup or lambda binding.
use sens::{eval_program, eval_pinned_legacy_core_source, eval_pinned_reasoning_source, Session};

fn session() -> Session {
    let mut s = Session::default();
    eval_pinned_legacy_core_source("lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut s).unwrap();
    for (path, source) in [
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
        ("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp")),
        ("lib/world.lisp", include_str!("../../../lib/world.lisp")),
        ("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp")),
        ("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp")),
    ] {
        eval_pinned_reasoning_source(path, source, &mut s)
            .unwrap_or_else(|e| panic!("bootstrap {path}: {e:?}"));
    }
    s
}

fn observe(s: &mut Session, source: &str) -> String {
    eval_program(source, s)
        .unwrap_or_else(|e| panic!("filesystem witness {source}: {e:?}"))
        .value.to_string()
}

#[test]
fn journal_event_fields_are_retrieved_from_a_real_alist() {
    let mut s = session();
    for (field, expected) in [("format","wsm-fs-event"), ("op","future-op")] {
        let source = format!("(fs-package-field (quote {field}) (fs-journal-event (quote future-op) (quote ())))");
        assert_eq!(observe(&mut s, &source), expected, "assoc field {field}");
    }
    assert_eq!(
        observe(&mut s, "(fs-package-field (quote missing) (fs-journal-event (quote future-op) (quote ())))"),
        "()",
        "unknown envelope keys remain absent, never synthesized"
    );
}

#[test]
fn object_package_and_nonempty_map_addresses_are_structurally_consistent() {
    let mut session = session();
    assert_eq!(
        observe(&mut session, "(car (fs-object-package-decision (fs-object-package (quote hello))))"),
        "accepted",
        "a canonical object envelope must be valid without foreign EQUAL semantics"
    );
    assert_eq!(
        observe(&mut session, "(fs-object-addresses (внутрішня-мапа-список (внутрішня-мапа-вставити \"one\" (quote payload) map-empty)))"),
        "(\"one\")",
        "AVL dotted entries must project a proper key list"
    );
}

#[test]
fn content_store_membership_is_an_exact_d1_bit_not_structural_one() {
    let mut session = session();
    for (source, expected) in [
        ("(content-store-contains? (content-store-put (empty-content-store) (quote star)) (knowledge-content-address (quote star)))", true),
        ("(content-store-contains? (empty-content-store) (knowledge-content-address (quote star)))", false),
        ("(fs-contains? (car (fs-write (fs-empty) \"x\" (quote star))) \"x\")", true),
        ("(fs-contains? (fs-empty) \"x\")", false),
    ] {
        let output = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("store D1 witness {source}: {e:?}"))
            .value;
        assert_eq!(output.as_predicate_bit(), Some(expected),
            "one-bit predicate must never be returned as structural (1)");
    }
}

#[test]
fn world_fourth_field_uses_d3_projection_not_unadmitted_d6_cadddr() {
    let mut session = session();
    for (source, expected) in [
        ("(world-metadata (empty-world))", "()"),
        ("(world-metadata (make-world (quote ()) (quote ()) (quote (marker))))", "(marker)"),
        ("(world-metadata (world-tell (make-world (quote ()) (quote ()) (quote (marker))) (quote zoo) (quote ((has-fur cat)))))", "(marker)"),
    ] {
        assert_eq!(observe(&mut session, source), expected,
            "World metadata is the fourth cons field, derived from D3");
    }
}

#[test]
fn world_dedup_source_can_export_its_own_knowledge_envelope() {
    let mut session = session();
    let src = "(let ((source (world-tell (empty-world) (quote zoo) (quote ((has-fur cat)))))) (car (make-world-knowledge-package source (quote zoo))))";
    assert_eq!(observe(&mut session, src), "(format . my-lisp-knowledge)");
}

#[test]
fn world_dedup_can_import_its_exported_knowledge_package() {
    let mut session = session();
    // The import returns (receipt world); the receipt is itself a tagged
    // record (accepted (module ...) (knowledge ...)), not a bare symbol.
    // Probe only its D3 CAR tag, retaining the richer language-owned receipt.
    let src = "(let ((source (world-tell (empty-world) (quote zoo) (quote ((has-fur cat)))))) (car (car (import-knowledge-package-world (empty-world) (make-world-knowledge-package source (quote zoo))))))";
    assert_eq!(observe(&mut session, src), "accepted",
        "an exported immutable world package must be reimportable");
}

#[test]
fn world_dedup_two_equal_world_contents_share_one_key() {
    let mut session = session();
    let src = "(let ((source (world-tell (empty-world) (quote zoo) (quote ((has-fur cat)))))) (let ((copy (second (import-knowledge-package-world (empty-world) (make-world-knowledge-package source (quote zoo)))))) (content-store-size (content-store-put-world (content-store-put-world (empty-content-store) source) copy))))";
    assert_eq!(observe(&mut session, src), "1");
}

#[test]
fn object_package_fold_consumes_every_valid_package() {
    let mut session = session();
    let source = "(let ((decision (fs-build-object-store (list (fs-object-package (quote first)) (fs-object-package (quote second))) (empty-content-store)))) (list (car decision) (content-store-size (second decision))))";
    assert_eq!(observe(&mut session, source), "(accepted 2)",
        "nonempty packages may not be accepted without insertion");
}

#[test]
fn object_package_fold_rejects_invalid_second_package_without_partial_success() {
    let mut session = session();
    let source = "(fs-build-object-store (list (fs-object-package (quote first)) (list (cons (quote format) (quote wsm-fs-object)) (cons (quote version) (quote (0 1))) (cons (quote address) \"incorrect\") (cons (quote value) (quote second)))) (empty-content-store))";
    assert_eq!(observe(&mut session, source), "(rejected address-mismatch)");
}

#[test]
fn object_package_fold_rejects_an_improper_tail() {
    let mut session = session();
    let source = "(fs-build-object-store (cons (fs-object-package (quote first)) (quote broken-tail)) (empty-content-store))";
    assert_eq!(observe(&mut session, source), "(rejected invalid-object-packages)");
}

#[test]
fn referenced_root_can_project_its_real_object_and_binding_lists() {
    let mut session = session();
    let prefix = "(let* ((value (quote (hello world))) (written (fs-write (fs-empty) \"notes/today\" value)) (fs (car written)) (root (fs-root-package fs)))";
    let source = format!("{prefix} (list (fs-package-field (quote objects) root) (fs-package-field (quote bindings) root)))");
    assert_eq!(
        observe(&mut session, &source),
        "((\"(hello world)\") ((\"notes/today\" . \"(hello world)\")))"
    );
}

#[test]
fn referenced_root_has_both_address_lists_in_its_supplied_store() {
    let mut session = session();
    let prefix = "(let* ((value (quote (hello world))) (written (fs-write (fs-empty) \"notes/today\" value)) (fs (car written)) (root (fs-root-package fs)))";
    let source = format!("{prefix} (list (fs-all-addresses-present? (fs-package-field (quote objects) root) (fs-objects fs)) (fs-binding-addresses-present? (fs-package-field (quote bindings) root) (fs-objects fs))))");
    let result = eval_program(&source, &mut session)
        .unwrap_or_else(|e| panic!("referenced root D1 validators {source}: {e:?}")).value.to_string();
    assert_eq!(result, "(1 1)");
}

#[test]
fn referenced_root_rebuilds_its_binding_map_without_nil_tail_access() {
    let mut session = session();
    let source = "(let* ((value (quote (hello world))) (written (fs-write (fs-empty) \"notes/today\" value)) (root (fs-root-package (car written)))) (внутрішня-мапа-отримати \"notes/today\" (fs-bindings-from-list (fs-package-field (quote bindings) root) map-empty)))";
    assert_eq!(observe(&mut session, source), "(\"(hello world)\")");
}

#[test]
fn referenced_root_decisions_isolate_the_rejected_stage() {
    let mut session = session();
    let prefix = "(let* ((value (quote (hello world))) (written (fs-write (fs-empty) \"notes/today\" value)) (fs (car written)) (root (fs-root-package fs)) (object (fs-object-package value)))";
    let cases = [
        ("(fs-root-package-decision root)", "accepted"),
        ("(fs-object-package-decision object)", "accepted"),
        ("(fs-build-object-store (list object) (empty-content-store))", "accepted"),
        ("(fs-reconstruct-root root (list object))", "accepted"),
    ];
    for (form, expected_tag) in cases {
        let expression = format!("{prefix} (car {form}))");
        let actual = observe(&mut session, &expression);
        assert_eq!(actual, expected_tag, "decision diverged at {form}; use full envelopes to locate rejection");
    }
}

#[test]
fn referenced_root_rebuild_is_a_separate_atomic_witness() {
    let mut session = session();
    let source = "(let* ((value (quote (hello world))) (written (fs-write (fs-empty) \"notes/today\" value)) (fs (car written)) (root (fs-root-package fs)) (object (fs-object-package value))) (car (fs-reconstruct-root root (list object))))";
    assert_eq!(observe(&mut session, source), "accepted");
}

#[test]
fn one_object_store_has_a_bounded_reconstruction_witness() {
    let mut session = session();
    assert_eq!(
        observe(&mut session,
            "(car (fs-build-object-store (list (fs-object-package (quote hello))) (empty-content-store)))"),
        "accepted",
        "one canonical object envelope should enter the immutable content store"
    );
    assert_eq!(
        observe(&mut session,
            "(car (fs-reconstruct-root (fs-root-package (fs-empty)) (list (fs-object-package (quote hello)))))"),
        "accepted",
        "an unreferenced but valid object must not corrupt an otherwise empty root"
    );
}

#[test]
fn empty_root_can_be_reconstructed_without_object_packages() {
    let mut session = session();
    assert_eq!(
        observe(&mut session, "(car (fs-reconstruct-root (fs-root-package (fs-empty)) (quote ())))"),
        "accepted",
        "an empty root has no missing-address proof obligation"
    );
}

#[test]
fn referenced_address_checks_are_d1_and_fail_closed() {
    let mut session = session();
    for (source, expected) in [
        ("(fs-all-addresses-present? (quote ()) (empty-content-store))", true),
        ("(fs-all-addresses-present? (quote missing) (empty-content-store))", false),
        ("(fs-all-addresses-present? (quote (missing)) (empty-content-store))", false),
        ("(fs-binding-addresses-present? (quote ()) (empty-content-store))", true),
        ("(fs-binding-addresses-present? (quote ((a . missing))) (empty-content-store))", false),
    ] {
        let actual = eval_program(source, &mut session)
            .unwrap_or_else(|e| panic!("D1 address validation {source}: {e:?}"))
            .value;
        assert_eq!(actual.as_predicate_bit(), Some(expected),
            "the filesystem must never use host T/NIL as a D1 result");
    }
}

#[test]
fn root_proper_list_validation_is_exact_d1_and_rejects_dotted_tails() {
    let mut session = session();
    for (value, expected) in [
        ("()", true),
        ("(a b)", true),
        ("(a . b)", false),
        ("a", false),
    ] {
        let expr = format!("(внутрішній-правильний-список? (quote {value}))");
        let actual = eval_program(&expr, &mut session)
            .unwrap_or_else(|e| panic!("D1 proper list predicate {expr}: {e:?}"))
            .value;
        assert_eq!(actual.as_predicate_bit(), Some(expected),
            "proper list checks must never consume host T/NIL or force empty tails");
    }
}

#[test]
fn world_module_membership_is_d1_and_not_unary_cons() {
    let mut session = session();
    for (src, expected) in [
        ("(world-module-missing? (empty-world) (quote zoo))", true),
        ("(world-module-known? (empty-world) (quote zoo))", false),
        ("(world-module-known? (world-tell (empty-world) (quote zoo) (quote ((has-fur cat)))) (quote zoo))", true),
    ] {
        let result = eval_program(src, &mut session)
            .unwrap_or_else(|e| panic!("world module membership {src}: {e:?}")).value;
        assert_eq!(result.as_predicate_bit(), Some(expected), "{src}");
    }
}

#[test]
fn world_module_name_symbol_law_is_d1_and_rejects_non_symbols() {
    let mut session = session();
    for (value, expected) in [
        ("(quote zoo)", true),
        ("\"zoo\"", false),
        ("7", false),
        ("(quote (zoo))", false),
        ("(quote ())", false),
    ] {
        let code = format!("(внутрішній-символ? {value})");
        let observed = eval_program(&code, &mut session)
            .unwrap_or_else(|e| panic!("world module name {code}: {e:?}")).value;
        assert_eq!(observed.as_predicate_bit(), Some(expected), "{code}");
    }
}

#[test]
fn world_knowledge_package_version_is_pair_safe_and_exact_d1() {
    let mut session = session();
    for (left, right, expected) in [
        ("(0 1)", "(0 1)", true),
        ("(0 1)", "(0 2)", false),
        ("(0 1)", "(0)", false),
        ("(0 1)", "()", false),
    ] {
        let code = format!(
            "(внутрішня-рівність-версій (quote {left}) (quote {right}))"
        );
        let value = eval_program(&code, &mut session)
            .unwrap_or_else(|e| panic!("pair-safe package version {code}: {e:?}"))
            .value;
        assert_eq!(value.as_predicate_bit(), Some(expected), "{code}");
    }
}

#[test]
fn root_commit_equality_is_structural_and_d1_only() {
    let mut session = session();
    for (a,b,expected) in [
        ("(a (b . c))", "(a (b . c))", true),
        ("(a (b . c))", "(a (b . d))", false),
        ("()", "()", true),
        ("()", "(a)", false),
        ("(a)", "()", false),
    ] {
        let source = format!("(внутрішня-рівність-структур (quote {a}) (quote {b}))");
        let value = eval_program(&source, &mut session)
            .unwrap_or_else(|e| panic!("structural D1 comparison {source}: {e:?}"))
            .value;
        assert_eq!(value.as_predicate_bit(), Some(expected),
            "root commit must compare full data, not SHA or host truth");
    }
}

#[test]
fn root_envelope_accepts_its_own_schema_without_object_only_fields() {
    let mut session = session();
    assert_eq!(
        observe(&mut session, "(car (fs-root-package-decision (fs-root-package (fs-empty))))"),
        "accepted",
        "root metadata consists of format/version/revision/bindings/objects, not an address/value pair"
    );
    assert_eq!(
        observe(&mut session, "(fs-package-field (quote address) (fs-root-package (fs-empty)))"),
        "()",
        "the absent object address remains absent in a root envelope"
    );
}

#[test]
fn empty_object_store_uses_an_admitted_two_argument_lisp_closure() {
    let mut s = session();
    assert_eq!(
        observe(&mut s, "(fs-build-object-store (quote ()) (empty-content-store))"),
        "(accepted ())",
        "nested reconstruction must not shadow the two-argument closure"
    );
}

#[test]
fn derived_macro_binds_list_values_as_one_variable() {
    let mut s = session();
    assert_eq!(
        observe(&mut s, "(нехай-похідне ((x (list 4 5))) (car x))"),
        "4"
    );
    assert_eq!(
        observe(&mut s, "(нехай-похідне ((x (fs-build-object-store (quote ()) (empty-content-store)))) (car x))"),
        "accepted"
    );
}
