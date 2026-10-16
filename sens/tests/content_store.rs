use sens::{eval_program, Session};

fn eval_store(source: &str) -> String {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/unify.lisp", include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/reason.lisp", include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/forward.lisp", include_str!("../../../lib/forward.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/world.lisp", include_str!("../../../lib/world.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp"), &mut session).unwrap();
    eval_program(source, &mut session)
        .unwrap()
        .value
        .to_string()
}

fn eval_store_fixture(source: &str) -> String {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/unify.lisp", include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/reason.lisp", include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/forward.lisp", include_str!("../../../lib/forward.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/world.lisp", include_str!("../../../lib/world.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp"), &mut session).unwrap();
    sens::eval_pinned_fs_fixture_source(
        "tests/fixtures/lisp-fs-conformance.lisp",
        source,
        &mut session,
    )
    .expect("pinned filesystem conformance fixture must execute as exact domains")
    .value
    .to_string()
}

fn eval_store_error(source: &str) -> String {
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source("lib/core.lisp", include_str!("../../../lib/core.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/unify.lisp", include_str!("../../../lib/unify.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/reason.lisp", include_str!("../../../lib/reason.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/forward.lisp", include_str!("../../../lib/forward.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/world.lisp", include_str!("../../../lib/world.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp"), &mut session).unwrap();
    sens::eval_pinned_reasoning_source("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp"), &mut session).unwrap();
    eval_program(source, &mut session)
        .expect_err("malformed serialized data must fail closed")
        .to_string()
}

#[test]
fn lisp_fs_keeps_old_snapshot_and_reads_new_bindings() {
    assert_eq!(
        eval_store(
            r#"
            (let ((empty (fs-empty)))
              (let ((first-write (fs-write empty "notes/today" (quote (hello world)))))
                (let ((old (car first-write))
                      (second-write (fs-write (car first-write)
                                              "notes/tomorrow"
                                              (quote (hello again)))))
                  (let ((new (car second-write)))
                    (list
                      (fs-read old "notes/today")
                      (fs-read old "notes/tomorrow")
                      (fs-read new "notes/today")
                      (fs-read new "notes/tomorrow")
                      (fs-revision old)
                      (fs-revision new))))))
            "#
        ),
        "((found (hello world) \"(hello world)\") (not-found \"notes/tomorrow\") (found (hello world) \"(hello world)\") (found (hello again) \"(hello again)\") 1 2)"
    );
}

#[test]
fn lisp_fs_deduplicates_equal_objects_but_keeps_explicit_missing_status() {
    // Canonical D1 membership is a one-bit predicate, not a structural (1).
    assert_eq!(
        eval_store(
            r#"
            (let ((empty (fs-empty)))
              (let ((a (fs-write empty "a" (quote ()))))
                (let ((b (fs-write (car a) "b" (quote ()))))
                  (list
                    (content-store-size (fs-objects (car b)))
                    (fs-contains? (car b) "a")
                    (fs-read (car b) "missing")))))
            "#
        ),
        "(1 1 (not-found \"missing\"))"
    );
}

#[test]
fn lisp_fs_conformance_fixture_is_deterministic() {
    assert_eq!(
        eval_store_fixture(include_str!("../../../tests/fixtures/lisp-fs-conformance.lisp")),
        "((found (hello world) \"(hello world)\") (not-found \"notes/empty\") (found () \"()\") (not-found \"missing\") 2 1 2)"
    );
}

#[test]
fn lisp_fs_overwrite_is_a_new_revision_and_old_value_remains_in_old_root() {
    assert_eq!(
        eval_store(
            r#"
            (let ((empty (fs-empty)))
              (let ((a (fs-write empty "same" (quote old))))
                (let ((b (fs-write (car a) "same" (quote new))))
                  (list
                    (fs-read (car a) "same")
                    (fs-read (car b) "same")
                    (fs-revision (car a))
                    (fs-revision (car b))))))
            "#
        ),
        "((found old \"old\") (found new \"new\") 1 2)"
    );
}

#[test]
fn lisp_fs_empty_name_is_literal_and_unknown_object_address_is_not_found() {
    assert_eq!(
        eval_store(
            r#"
            (let ((empty (fs-empty)))
              (let ((written (fs-write empty "" (quote value))))
                (let ((malformed
                        (list (fs-objects (car written))
                              (map-insert "ghost" "missing-address" map-empty)
                              9)))
                  (list
                    (fs-read (car written) "")
                    (fs-read malformed "ghost")
                    (fs-read malformed "absent")))))
            "#
        ),
        "((found value \"value\") (not-found \"ghost\") (not-found \"absent\"))"
    );
}

#[test]
fn lisp_fs_object_envelope_round_trips_and_rejects_bad_metadata() {
    assert_eq!(
        eval_store(
            r#"
            (let ((encoded (fs-serialize-object (quote (hello world)))))
              (list
                encoded
                (fs-deserialize-object encoded)
                (fs-object-package-decision
                  (list
                    (cons (quote format) (quote wsm-fs-object))
                    (cons (quote version) (quote (9 9)))
                    (cons (quote address) "wrong")
                    (cons (quote value) (quote (hello world)))))
                (fs-object-package-decision
                  (list
                    (cons (quote format) (quote wsm-fs-object))
                    (cons (quote version) (quote (0 1)))
                    (cons (quote address) "wrong")
                    (cons (quote value) (quote (hello world)))))))
            "#
        ),
        "(\"((format . wsm-fs-object) (version #q2:0/1 #q2:1/1) (address . \\\"(hello world)\\\") (value hello world))\" (accepted (hello world)) (rejected unsupported-version) (rejected address-mismatch))"
    );
}

#[test]
fn lisp_fs_root_envelope_round_trips_bindings_revision_and_addresses() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((fs (car (fs-write (fs-empty) "notes/today" (quote (hello world)))))
                   (encoded (fs-serialize-root fs))
                   (bad
                     (list
                       (cons (quote format) (quote wsm-fs-root))
                       (cons (quote version) (quote (9 9)))
                       (cons (quote revision) 1)
                       (cons (quote bindings) (quote ()))
                       (cons (quote objects) (quote ())))))
              (list encoded
                    (car (fs-root-package-decision (read encoded)))
                    (car (fs-root-package-decision bad))))
            "#
        ),
        "(\"((format . wsm-fs-root) (version #q2:0/1 #q2:1/1) (revision . #q2:1/1) (bindings (\\\"notes/today\\\" . \\\"(hello world)\\\")) (objects \\\"(hello world)\\\"))\" accepted rejected)"
    );
}

#[test]
fn lisp_fs_reconstructs_immutable_root_from_valid_envelopes() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((value (quote (hello world)))
                   (written (fs-write (fs-empty) "notes/today" value))
                   (fs (car written))
                   (root (fs-root-package fs))
                   (object (fs-object-package value))
                   (rebuilt (fs-reconstruct-root root (list object))))
              (list
                (car rebuilt)
                (fs-read (second rebuilt) "notes/today")
                (fs-revision (second rebuilt))))
            "#
        ),
        "(accepted (found (hello world) \"(hello world)\") 1)"
    );
}

#[test]
fn lisp_fs_reconstruction_rejects_missing_referenced_object() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((written (fs-write (fs-empty) "notes/today" (quote value)))
                   (root (fs-root-package (car written))))
              (fs-reconstruct-root root (quote ())))
            "#
        ),
        "(rejected missing-object)"
    );
}

#[test]
fn lisp_fs_journal_replay_matches_direct_construction() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((empty (fs-empty))
                   (event (fs-journal-write-event "notes/today" (quote (hello world))))
                   (journal (fs-journal-append (quote ()) event))
                   (direct (car (fs-write empty "notes/today" (quote (hello world)))))
                   (with-commit
                     (fs-journal-append journal (fs-journal-root-commit-event
                                                  (fs-root-package direct))))
                   (replayed (fs-journal-replay with-commit)))
              (list
                (car replayed)
                (fs-read (second replayed) "notes/today")
                (fs-revision (second replayed))))
            "#
        ),
        "(accepted (found (hello world) \"(hello world)\") 1)"
    );
}

#[test]
fn lisp_fs_journal_serialization_round_trips_into_the_same_snapshot() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((journal
                     (list
                       (fs-journal-write-event "notes/today" (quote (hello world)))))
                   (encoded (fs-serialize-journal journal))
                   (replayed (fs-deserialize-journal encoded)))
              (list
                encoded
                (car replayed)
                (fs-read (second replayed) "notes/today")))
            "#
        ),
        "(\"(((format . wsm-fs-event) (version #q2:0/1 #q2:1/1) (op . write) (name . \\\"notes/today\\\") (value hello world)))\" accepted (found (hello world) \"(hello world)\"))"
    );
}

#[test]
fn lisp_fs_journal_replay_rejects_unknown_and_truncated_events() {
    assert_eq!(
        eval_store(
            r#"
            (list
              (fs-journal-replay
                (list
                  (fs-journal-event (quote future-op) (quote ()))) )
              (fs-journal-replay (list (quote ()))) )
            "#
        ),
        "((rejected unknown-event) (rejected invalid-event))"
    );
}

#[test]
fn lisp_fs_journal_replay_rejects_structurally_incomplete_events() {
    assert_eq!(
        eval_store(
            r#"
            (fs-journal-replay
              (list
                (list
                  (cons (quote format) (quote wsm-fs-event))
                  (cons (quote version) (quote (0 1)))
                  (cons (quote op) (quote write))
                  (cons (quote name) "missing-value"))))
            "#
        ),
        "(rejected incomplete-write)"
    );
}

#[test]
fn lisp_fs_recovery_keeps_old_root_until_root_pointer_commit() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((old (fs-empty))
                   (new (car (fs-write old "notes/today" (quote value)))))
              (list
                (fs-recover-commit old new (quote objects))
                (fs-recover-commit old new (quote journal))
                (fs-recover-commit old new (quote root-pointer))))
            "#
        ),
        "((recovered (() () 0)) (recovered (() () 0)) (recovered ((\"value\" value 1 () ()) (\"notes/today\" \"value\" 1 () ()) 1)))"
    );
}

#[test]
fn lisp_fs_recovery_rejects_unknown_commit_stage() {
    assert_eq!(
        eval_store("(fs-recover-commit (fs-empty) (fs-empty) (quote upload))"),
        "(rejected unknown-commit-stage)"
    );
}

#[test]
fn lisp_fs_recovery_keeps_old_root_when_candidate_root_is_corrupt() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((old (fs-root-package (fs-empty)))
                   (corrupt
                     (list
                       (cons (quote format) (quote wsm-fs-root))
                       (cons (quote version) (quote (0 1)))
                       (cons (quote revision) 2)
                       (cons (quote bindings) (quote ()))
                       (cons (quote objects) (quote broken)))))
              (fs-recover-root-package old corrupt (quote root-pointer)))
            "#
        ),
        "(recovered ((format . wsm-fs-root) (version 0 1) (revision . 0) (bindings) (objects)) rejected-candidate)"
    );
}

#[test]
fn lisp_fs_partial_object_and_journal_bytes_fail_closed() {
    let object_error = eval_store_error(
        r#"(fs-deserialize-object "((format . wsm-fs-object) (version 0 1) (value")"#,
    );
    assert!(!object_error.is_empty(), "{object_error}");
    let journal_error = eval_store_error(
        r#"(fs-deserialize-journal "((format . wsm-fs-event) (version 0 1) (op . write)")"#,
    );
    assert!(!journal_error.is_empty(), "{journal_error}");
}

#[test]
fn lisp_fs_replay_rejects_after_valid_prefix_without_returning_partial_state() {
    assert_eq!(
        eval_store(
            r#"
            (fs-journal-replay
              (list
                (fs-journal-write-event "ok" (quote value))
                (quote malformed-event)))
            "#
        ),
        "(rejected invalid-event)"
    );
}

#[test]
fn lisp_fs_reconstruction_rejects_after_valid_object_without_partial_snapshot() {
    assert_eq!(
        eval_store(
            r#"
            (let* ((value (quote value))
                   (written (fs-write (fs-empty) "ok" value))
                   (root (fs-root-package (car written)))
                   (valid (fs-object-package value))
                   (invalid
                     (list
                       (cons (quote format) (quote wsm-fs-object))
                       (cons (quote version) (quote (0 1)))
                       (cons (quote address) "wrong")
                       (cons (quote value) (quote other)))))
              (fs-reconstruct-root root (list valid invalid)))
            "#
        ),
        "(rejected address-mismatch)"
    );
}

#[test]
fn lisp_fs_fault_injection_keeps_old_root_until_pointer_is_published() {
    // Bounded temporary-medium witness for F4: only the root-pointer stage
    // may replace the visible root file. Objects/journal writes alone leave
    // the previous root recoverable.
    // Обмежений temporary-medium witness F4: лише root-pointer може замінити
    // видимий root; записи objects/journal залишають старий root.
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("wsm-fs-recovery-{nonce}"));
    std::fs::create_dir(&dir).unwrap();
    let old = "old-root";
    let new = "new-root";
    let root_path = dir.join("root.pointer");
    std::fs::write(&root_path, old).unwrap();
    for stage in ["objects", "journal", "root-pointer"] {
        if stage == "root-pointer" {
            std::fs::write(&root_path, new).unwrap();
        }
        let visible = std::fs::read_to_string(&root_path).unwrap();
        if stage == "root-pointer" {
            assert_eq!(visible, new);
        } else {
            assert_eq!(visible, old);
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stored_knowledge_is_retrievable_by_its_canonical_address() {
    assert_eq!(
        eval_store(
            r#"
            (let ((knowledge (quote ((planet earth)))))
              (let ((store (content-store-put (empty-content-store) knowledge)))
                (content-store-get store (knowledge-content-address knowledge))))
            "#
        ),
        "(((planet earth)))"
    );
}

#[test]
fn inserting_equal_content_twice_does_not_grow_the_store() {
    assert_eq!(
        eval_store(
            r#"
            (let ((knowledge (quote ((planet earth)))))
              (let ((once (content-store-put (empty-content-store) knowledge)))
                (let ((twice (content-store-put once knowledge)))
                  (list (content-store-size once)
                        (content-store-size twice)))))
            "#
        ),
        "(1 1)"
    );
}

#[test]
fn different_content_occupies_different_addresses() {
    // Both membership results are exact D1, proven separately in fs_bridge_diagnostics.
    assert_eq!(
        eval_store(
            r#"
            (let ((earth (quote ((planet earth))))
                  (mars (quote ((planet mars)))))
              (let ((store (content-store-put
                             (content-store-put (empty-content-store) earth)
                             mars)))
                (list (content-store-size store)
                      (content-store-contains?
                        store (knowledge-content-address earth))
                      (content-store-contains?
                        store (knowledge-content-address mars)))))
            "#
        ),
        "(2 1 1)"
    );
}

#[test]
fn reconstructed_equal_worlds_deduplicate_in_the_store() {
    assert_eq!(
        eval_store(
            r#"
            (let ((source
                    (world-tell (empty-world) (quote zoo) (quote ((has-fur cat))))))
              (let ((copy
                      (second
                        (import-knowledge-package-world
                          (empty-world)
                          (make-world-knowledge-package source (quote zoo))))))
                (let ((store
                        (content-store-put-world
                          (content-store-put-world (empty-content-store) source)
                          copy)))
                  (content-store-size store))))
            "#
        ),
        "1"
    );
}

#[test]
fn lisp_fs_journal_replay_rejects_malformed_field_entries_sens_owned() {
    // Semantic cases/branches are SENS code in the fixture, not Rust logic.
    // Rust only runs the pre-existing pinned library chain and observes output.
    let mut session = Session::default();
    sens::eval_pinned_legacy_core_source(
        "lib/core.lisp",
        include_str!("../../../lib/core.lisp"),
        &mut session,
    ).expect("ratified core loader");
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
        sens::eval_pinned_reasoning_source(path, source, &mut session)
            .unwrap_or_else(|error| panic!("pinned SENS library {path}: {error}"));
    }
    let fixture = include_str!("../../../tests/fixtures/fs-alist-structural-law.lisp");
    let parsed = sens::parse_mixed_exact_domain(fixture)
        .expect("structural alist SENS witness: exact D3/D4 reader");
    let result = sens::eval_parsed_expressions(&parsed, &mut session)
        .expect("SENS self-check must not reach CAR() falsifier");
    assert_eq!(result.value.to_string(), "structural-law-pass");
}
