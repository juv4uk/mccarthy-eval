//! #2442 — shared-location mutation attacked by explicit D4 state passing.
//!
//! Research only. No production mutation operator and no post-D4 identity.
//!
//! The transformed program separates:
//!   lexical name -> explicit location data
//!   location -> value in an immutable explicit store
//!
//! "Mutation" becomes ordinary data transformation returning a new store.
//! Pre-existing observers are transformed closures that receive the current
//! store explicitly. No Rc<RefCell<_>>, host side table, global mutable state,
//! exception, or continuation object is used.

use sens::{eval_program, load_core_library, Session};

const DERIVED_STATE: &str = r#"
(00001001 post-d4-alist-find
  (00001000 (key rows)
    (00000111
      ((00000010 rows) ()
       (00000001 (missing)))
      ((00000010 rows) (0)
       (00000111
         ((00000011 key (00000101 (00000101 rows))) (1)
          (00000100
            (00000001 found)
            (00000100
              (00000101 (00000110 (00000101 rows)))
              (00000001 ()))))
         ((00000011 key key) (1)
          (post-d4-alist-find key (00000110 rows))))))))

(00001001 post-d4-resolve-location
  (00001000 (name frames)
    (00000111
      ((00000010 frames) ()
       (00000001 (missing)))
      ((00000010 frames) (0)
       ((00001000 (here)
          (00000111
            ((00000011 (00000101 here) (00000001 found)) (1)
             here)
            ((00000011 (00000101 here) (00000001 missing)) (1)
             (post-d4-resolve-location name (00000110 frames)))))
        (post-d4-alist-find name (00000101 frames)))))))

(00001001 post-d4-store-read
  (00001000 (location store)
    (00000101
      (00000110
        (post-d4-alist-find location store)))))

(00001001 post-d4-store-update-existing
  (00001000 (location value store)
    (00000111
      ((00000011 location (00000101 (00000101 store))) (1)
       (00000100
         (00000100
           location
           (00000100 value (00000001 ())))
         (00000110 store)))
      ((00000011 location location) (1)
       (00000100
         (00000101 store)
         (post-d4-store-update-existing
           location
           value
           (00000110 store)))))))

(00001001 post-d4-state-set
  (00001000 (name value frames store)
    ((00001000 (resolved)
       (00000111
         ((00000011 (00000101 resolved) (00000001 found)) (1)
          (00000100
            (00000001 ok)
            (00000100
              (post-d4-store-update-existing
                (00000101 (00000110 resolved))
                value
                store)
              (00000001 ()))))
         ((00000011 (00000101 resolved) (00000001 missing)) (1)
          (00000001 (error unbound-location)))))
     (post-d4-resolve-location name frames))))

(00001001 post-d4-make-observer
  (00001000 (location)
    (00001000 (store)
      (post-d4-store-read location store))))

(00001001 post-d4-call-observer
  (00001000 (observer store)
    (observer store)))
"#;

fn session() -> Session {
    let mut s = Session::default();
    load_core_library(&mut s).expect("core library should load");
    eval_program(DERIVED_STATE, &mut s).expect("explicit-state D4 model should load");
    s
}

fn run(s: &mut Session, source: &str) -> String {
    eval_program(source, s)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn observer_created_before_update_sees_new_value_when_given_new_store() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (frames store)
           ((00001000 (resolved)
              ((00001000 (observer)
                 ((00001000 (set-result)
                    ((00001000 (new-store)
                       (00000100
                         (observer store)
                         (00000100
                           (observer new-store)
                           (00000001 ()))))
                     (00000101 (00000110 set-result))))
                  (post-d4-state-set
                    (00000001 x)
                    (00000001 new)
                    frames
                    store)))
               (post-d4-make-observer
                 (00000101 (00000110 resolved)))))
            (post-d4-resolve-location (00000001 x) frames)))
         (00000001 (((x l0))))
         (00000001 ((l0 old))))
        "#,
    );

    assert_eq!(result, "(old new)");
}

#[test]
fn higher_order_transport_and_aliases_observe_the_same_explicit_location() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (frames store)
           ((00001000 (xloc yloc)
              ((00001000 (xobserver yobserver)
                 ((00001000 (set-result)
                    ((00001000 (new-store)
                       (00000100
                         (post-d4-call-observer xobserver new-store)
                         (00000100
                           (post-d4-call-observer yobserver new-store)
                           (00000001 ()))))
                     (00000101 (00000110 set-result))))
                  (post-d4-state-set
                    (00000001 x)
                    (00000001 new)
                    frames
                    store)))
               (post-d4-make-observer xloc)
               (post-d4-make-observer yloc)))
            (00000101
              (00000110
                (post-d4-resolve-location (00000001 x) frames)))
            (00000101
              (00000110
                (post-d4-resolve-location (00000001 alias) frames)))))
         (00000001 (((x l0) (alias l0))))
         (00000001 ((l0 old))))
        "#,
    );

    assert_eq!(result, "(new new)");
}

#[test]
fn nested_shadowing_updates_only_the_nearest_explicit_location() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        ((00001000 (frames store)
           ((00001000 (leaf-loc root-loc)
              ((00001000 (leaf-observer root-observer)
                 ((00001000 (set-result)
                    ((00001000 (new-store)
                       (00000100
                         (root-observer new-store)
                         (00000100
                           (leaf-observer new-store)
                           (00000001 ()))))
                     (00000101 (00000110 set-result))))
                  (post-d4-state-set
                    (00000001 x)
                    (00000001 middle-new)
                    frames
                    store)))
               (post-d4-make-observer leaf-loc)
               (post-d4-make-observer root-loc)))
            (00000101
              (00000110
                (post-d4-resolve-location (00000001 x) frames)))
            (00000001 l0)))
         (00000001 (() ((x l1)) ((x l0))))
         (00000001 ((l0 root-old) (l1 middle-old))))
        "#,
    );

    assert_eq!(result, "(root-old middle-new)");
}

#[test]
fn missing_name_fails_closed_without_constructing_a_binding() {
    let mut s = session();

    let result = run(
        &mut s,
        r#"
        (post-d4-state-set
          (00000001 missing)
          (00000001 new)
          (00000001 (((x l0))))
          (00000001 ((l0 old))))
        "#,
    );

    assert_eq!(result, "(error unbound-location)");
}

#[test]
fn derivation_uses_only_existing_d3_d4_semantic_primitives() {
    const ALLOWED: &[&str] = &[
        "00000001", // QUOTE
        "00000010", // ATOM
        "00000011", // EQ
        "00000100", // CONS
        "00000101", // CAR
        "00000110", // CDR
        "00000111", // COND
        "00001000", // LAMBDA
        "00001001", // DEFINE
    ];

    for token in DERIVED_STATE.split(|c: char| c != '0' && c != '1') {
        if token.len() == 8 {
            assert!(
                ALLOWED.contains(&token),
                "unexpected non-D3/D4 identity in state-passing derivation: {token}"
            );
        }
    }

    for forbidden in ["SETQ", " SET ", "RefCell", "Rc<", "PROG", "RETURN", "FEXPR", "FSUBR"] {
        assert!(
            !DERIVED_STATE.contains(forbidden),
            "derived source must not import hidden state/control surface {forbidden}"
        );
    }
}
