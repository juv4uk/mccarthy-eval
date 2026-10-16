//! Recovery witness for #3924.
//!
//! Research-only: this proves current ENV reflection behavior. It does not
//! ratify D8 or make 00000000 a production resident.

use sens::{eval_program, Session, Value};

fn env_lookup(snapshot: &Value, name: &str) -> Option<Value> {
    let mut cursor = snapshot;
    loop {
        match cursor {
            Value::Nil => return None,
            Value::Pair(entry, tail) => {
                let Value::Pair(key, value) = entry.as_ref() else {
                    panic!("ENV snapshot entry must be a dotted pair");
                };
                if matches!(key.as_ref(), Value::String(text) if text.as_ref() == name) {
                    return Some(value.as_ref().clone());
                }
                cursor = tail.as_ref();
            }
            other => panic!("ENV snapshot must be a proper list, got {other}"),
        }
    }
}

fn env_names(snapshot: &Value) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = snapshot;
    loop {
        match cursor {
            Value::Nil => return names,
            Value::Pair(entry, tail) => {
                let Value::Pair(key, _) = entry.as_ref() else {
                    panic!("ENV snapshot entry must be a dotted pair");
                };
                let Value::String(text) = key.as_ref() else {
                    panic!("ENV snapshot key must be a string");
                };
                names.push(text.to_string());
                cursor = tail.as_ref();
            }
            other => panic!("ENV snapshot must be a proper list, got {other}"),
        }
    }
}

#[test]
fn env_reflection_enumerates_visible_binding() {
    let mut session = Session::default();
    let result = eval_program(
        "(define __d8_env_probe__ 41) (env)",
        &mut session,
    )
    .expect("ENV reflection should execute");

    let value = env_lookup(&result.value, "__d8_env_probe__")
        .expect("ENV must enumerate a newly visible binding");
    assert_eq!(value.to_string(), "41");
}

#[test]
fn env_reflection_resolves_shadowing_to_innermost_value() {
    let mut session = Session::default();
    let result = eval_program(
        "(define __d8_env_shadow__ 1) ((lambda (__d8_env_shadow__) (env)) 2)",
        &mut session,
    )
    .expect("ENV reflection should execute under a lexical child frame");

    let value = env_lookup(&result.value, "__d8_env_shadow__")
        .expect("shadowed binding must remain visible once");
    assert_eq!(value.to_string(), "2");
}

#[test]
fn env_reflection_observes_lexical_capture_not_dynamic_caller_shadow() {
    let mut session = Session::default();
    let result = eval_program(
        "
        (define __d8_make_probe__
          (lambda (__d8_captured__)
            (lambda () (env))))
        (define __d8_probe_closure__ (__d8_make_probe__ 11))
        ((lambda (__d8_captured__) (__d8_probe_closure__)) 22)
        ",
        &mut session,
    )
    .expect("captured lexical ENV reflection should execute");

    let value = env_lookup(&result.value, "__d8_captured__")
        .expect("captured lexical binding must be visible");
    assert_eq!(value.to_string(), "11");
}

#[test]
fn env_reflection_has_deterministic_name_order() {
    let mut session = Session::default();
    let result = eval_program(
        "(define __d8_zeta__ 1) (define __d8_alpha__ 2) (env)",
        &mut session,
    )
    .expect("ENV reflection should execute");

    let names = env_names(&result.value);
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "ENV snapshot order must be canonical by name");
}
