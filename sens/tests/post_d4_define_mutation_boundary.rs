//! #2314 — boundary between D4 DEFINE update and historical SET/SETQ mutation.
//!
//! Research-only. This does not add SET/SETQ or allocate any post-D4 identity.
//! It pins exactly what current D4 DEFINE can and cannot mutate observably.

use sens::{eval_program, load_core_library, ErrorKind, Session};

fn session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    session
}

fn eval(session: &mut Session, source: &str) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn define_redefinition_updates_a_global_binding_seen_by_existing_closure() {
    let mut s = session();

    let result = eval(
        &mut s,
        r#"
        (00001001 g (00000001 old))
        (00001001 watch-g (00001000 () g))
        (00001001 g (00000001 new))
        (watch-g)
        "#,
    );

    assert_eq!(result, "new");
}

#[test]
fn define_updates_an_existing_slot_in_the_current_lexical_frame() {
    let mut s = session();

    let result = eval(
        &mut s,
        r#"
        ((00001000 (x)
           (00001001 watch-x (00001000 () x))
           (00001001 x (00000001 new))
           (watch-x))
         (00000001 old))
        "#,
    );

    // DEFINE is already an observable in-place update when the target binding
    // is a slot of the current frame. A closure created before the update sees
    // NEW through the same captured frame.
    assert_eq!(result, "new");
}

#[test]
fn define_in_child_frame_shadows_inherited_binding_instead_of_updating_parent_slot() {
    let mut s = session();

    let result = eval(
        &mut s,
        r#"
        ((00001000 (x)
           (00001001 watch-x (00001000 () x))
           ((00001000 ()
              (00001001 x (00000001 new))))
           (watch-x))
         (00000001 old))
        "#,
    );

    // The inner DEFINE has no local x slot, so it constructs a new binding in
    // the child frame. The pre-existing observer of the parent x still sees OLD.
    assert_eq!(result, "old");
}

#[test]
fn define_target_is_syntax_fixed_not_runtime_selected() {
    let mut s = session();

    let error = eval_program(
        "(00001001 (00000001 x) (00000001 new))",
        &mut s,
    )
    .expect_err("DEFINE target must be a syntax-level symbol");

    assert_eq!(error.kind, ErrorKind::InvalidForm);
}

#[test]
fn mutation_lower_bound_is_narrower_than_generic_rebinding() {
    // Current evidence:
    //
    //   current-frame slot update  -> already D4 DEFINE
    //   global redefinition        -> already D4 DEFINE
    //   child shadows parent       -> D4 behavior, not parent-location update
    //   runtime-selected target    -> not expressible by DEFINE syntax
    //
    // Therefore #2314 must not claim "state transition" merely from seeing a
    // value change. The surviving SET/SETQ question is whether historical
    // semantics require updating an existing non-local location (and/or a
    // runtime-selected symbol) rather than constructing a shadow.
}
