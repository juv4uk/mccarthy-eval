//! #2402 — lower bound for historical SET/SETQ-style shared-location update.
//!
//! Research only. No production mutation operator and no post-D4 identity are added.
//!
//! The live SENS half pins the already-merged D4 boundary:
//! a DEFINE performed in a child frame shadows an inherited binding.
//!
//! The bounded countermodel then adds exactly one semantic delta:
//! update the nearest already-existing lexical location instead of constructing
//! a new child binding. Pre-existing observers distinguish the two models.

use sens::{eval_program, load_core_library, Session};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

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

type Location = Rc<RefCell<&'static str>>;
type FrameRef = Rc<RefCell<ModelFrame>>;

#[derive(Default)]
struct ModelFrame {
    bindings: HashMap<&'static str, Location>,
    parent: Option<FrameRef>,
}

fn root_frame() -> FrameRef {
    Rc::new(RefCell::new(ModelFrame::default()))
}

fn child_frame(parent: &FrameRef) -> FrameRef {
    Rc::new(RefCell::new(ModelFrame {
        bindings: HashMap::new(),
        parent: Some(Rc::clone(parent)),
    }))
}

fn define_local(frame: &FrameRef, name: &'static str, value: &'static str) -> Location {
    let location = Rc::new(RefCell::new(value));
    frame
        .borrow_mut()
        .bindings
        .insert(name, Rc::clone(&location));
    location
}

fn lookup_location(frame: &FrameRef, name: &str) -> Option<Location> {
    let mut current = Some(Rc::clone(frame));
    while let Some(frame) = current {
        let next = {
            let borrowed = frame.borrow();
            if let Some(location) = borrowed.bindings.get(name) {
                return Some(Rc::clone(location));
            }
            borrowed.parent.as_ref().map(Rc::clone)
        };
        current = next;
    }
    None
}

fn nearest_existing_update(
    frame: &FrameRef,
    name: &str,
    value: &'static str,
) -> Result<Location, &'static str> {
    let Some(location) = lookup_location(frame, name) else {
        return Err("unbound-location");
    };
    *location.borrow_mut() = value;
    Ok(location)
}

#[test]
fn live_d4_child_define_shadows_parent_location() {
    let mut s = session();

    let result = eval(
        &mut s,
        r#"
        ((00001000 (x)
           (00001001 before (00001000 () x))
           ((00001000 ()
              (00001001 x (00000001 new))
              x))
           (00000100
             (before)
             (00000100
               ((00001000 () x))
               (00000001 ()))))
         (00000001 old))
        "#,
    );

    assert_eq!(result, "(old old)");
}

#[test]
fn nearest_existing_update_changes_the_same_outer_location_for_existing_observers() {
    let root = root_frame();
    let parent_x = define_local(&root, "x", "old");
    let observer_before = Rc::clone(&parent_x);

    let child = child_frame(&root);
    assert!(!child.borrow().bindings.contains_key("x"));
    assert_eq!(*lookup_location(&child, "x").unwrap().borrow(), "old");

    let updated = nearest_existing_update(&child, "x", "new")
        .expect("an inherited x location should be found");

    assert!(Rc::ptr_eq(&updated, &parent_x));
    assert!(!child.borrow().bindings.contains_key("x"));
    assert_eq!(*observer_before.borrow(), "new");
    assert_eq!(*lookup_location(&child, "x").unwrap().borrow(), "new");

    let observer_after = lookup_location(&child, "x").unwrap();
    assert!(Rc::ptr_eq(&observer_before, &observer_after));
    assert_eq!(*observer_after.borrow(), "new");
}

#[test]
fn nearest_existing_update_selects_the_nearest_shadow_not_the_root_location() {
    let root = root_frame();
    let root_x = define_local(&root, "x", "root-old");

    let middle = child_frame(&root);
    let middle_x = define_local(&middle, "x", "middle-old");

    let leaf = child_frame(&middle);
    assert!(!leaf.borrow().bindings.contains_key("x"));

    let updated = nearest_existing_update(&leaf, "x", "middle-new")
        .expect("nearest middle x should exist");

    assert!(Rc::ptr_eq(&updated, &middle_x));
    assert!(!Rc::ptr_eq(&updated, &root_x));
    assert_eq!(*middle_x.borrow(), "middle-new");
    assert_eq!(*root_x.borrow(), "root-old");
    assert_eq!(*lookup_location(&leaf, "x").unwrap().borrow(), "middle-new");
    assert!(!leaf.borrow().bindings.contains_key("x"));
}

#[test]
fn nearest_existing_update_fails_closed_for_a_missing_name() {
    let root = root_frame();
    let child = child_frame(&root);

    assert_eq!(
        nearest_existing_update(&child, "missing", "new"),
        Err("unbound-location")
    );
    assert!(!child.borrow().bindings.contains_key("missing"));
    assert!(!root.borrow().bindings.contains_key("missing"));
}

#[test]
fn lower_bound_is_location_identity_not_generic_value_change() {
    let root = root_frame();
    let parent_x = define_local(&root, "x", "old");
    let observer_before = Rc::clone(&parent_x);

    let child = child_frame(&root);
    let child_shadow = define_local(&child, "x", "new");

    // A generic value change can make two bindings carry the same payload,
    // but that does not make them the same semantic location.
    *parent_x.borrow_mut() = "new";

    assert_eq!(*observer_before.borrow(), "new");
    assert_eq!(*child_shadow.borrow(), "new");
    assert!(!Rc::ptr_eq(&observer_before, &child_shadow));

    // Lookup from the child still resolves its distinct shadow location.
    let child_lookup = lookup_location(&child, "x").expect("child shadow must resolve");
    assert!(Rc::ptr_eq(&child_lookup, &child_shadow));
    assert!(!Rc::ptr_eq(&child_lookup, &observer_before));
}
