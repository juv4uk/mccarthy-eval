use sens::{eval_program, Session};

fn store_session() -> Session {
    let mut session = Session::default();
    for (path, source) in [
        ("lib/core.lisp", include_str!("../../../lib/core.lisp")),
        ("lib/unify.lisp", include_str!("../../../lib/unify.lisp")),
        ("lib/reason.lisp", include_str!("../../../lib/reason.lisp")),
        ("lib/forward.lisp", include_str!("../../../lib/forward.lisp")),
        ("lib/knowledge.lisp", include_str!("../../../lib/knowledge.lisp")),
        ("lib/persistent-map.lisp", include_str!("../../../lib/persistent-map.lisp")),
        ("lib/world.lisp", include_str!("../../../lib/world.lisp")),
        ("lib/content-store.lisp", include_str!("../../../lib/content-store.lisp")),
        ("lib/lisp-fs.lisp", include_str!("../../../lib/lisp-fs.lisp")),
    ] {
        if path == "lib/core.lisp" {
            sens::eval_pinned_legacy_core_source(path, source, &mut session)
                .expect("pinned historical core must load");
        } else if matches!(path, "lib/unify.lisp" | "lib/reason.lisp" | "lib/forward.lisp" | "lib/knowledge.lisp" | "lib/world.lisp" | "lib/persistent-map.lisp" | "lib/content-store.lisp" | "lib/lisp-fs.lisp") {
            sens::eval_pinned_reasoning_source(path, source, &mut session)
                .expect("pinned reasoning dependency must load");
        } else {
            eval_program(source, &mut session).expect("content-store dependency must load");
        }
    }
    session
}

fn observe(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .expect("content-store observation must execute")
        .value
        .to_string()
}

#[test]
fn content_store_mechanism_keeps_deterministic_images_and_distinct_history_entries() {
    let mut session = store_session();

    let root_image = r#"
        (let* ((value (quote (lambda (x) x)))
               (fs (car (fs-write (fs-empty) "code" value))))
          (fs-serialize-root fs))
    "#;
    eprintln!("CONTENT-STORE-PROBE: root_a BEGIN");
    let root_a = observe(&mut session, root_image);
    eprintln!("CONTENT-STORE-PROBE: root_a END => {root_a}");
    eprintln!("CONTENT-STORE-PROBE: root_b BEGIN");
    let root_b = observe(&mut session, root_image);
    eprintln!("CONTENT-STORE-PROBE: root_b END => {root_b}");
    assert_eq!(
        root_a, root_b,
        "repeating the same root serialization must preserve the exact image"
    );

    let object_image = "(fs-serialize-object (quote (lambda (x) x)))";
    eprintln!("CONTENT-STORE-PROBE: object_a BEGIN");
    let object_a = observe(&mut session, object_image);
    eprintln!("CONTENT-STORE-PROBE: object_a END => {object_a}");
    eprintln!("CONTENT-STORE-PROBE: object_b BEGIN");
    let object_b = observe(&mut session, object_image);
    eprintln!("CONTENT-STORE-PROBE: object_b END => {object_b}");
    assert_eq!(
        object_a, object_b,
        "repeating the same object serialization must preserve the exact image"
    );

    let history_cardinality = r#"
        (let ((direct
                (world-tell (empty-world) (quote zoo) (quote ((has-fur cat))))))
          (let ((retold
                  (world-tell
                    (world-retract
                      (world-tell (empty-world) (quote zoo) (quote ((has-fur cat))))
                      (quote zoo) (quote ((has-fur cat))))
                    (quote zoo) (quote ((has-fur cat))))))
            (let ((store
                    (content-store-put-world
                      (content-store-put-world (empty-content-store) direct)
                      retold)))
              (content-store-size store))))
    "#;
    eprintln!("CONTENT-STORE-PROBE: history_cardinality BEGIN");
    let history = observe(&mut session, history_cardinality);
    eprintln!("CONTENT-STORE-PROBE: history_cardinality END => {history}");
    assert_eq!(
        history,
        "2",
        "equal current projections with distinct histories must occupy two store entries"
    );
}
