//! #1413 / #1905: admitted human surfaces are lowering-only.
//!
//! Current binary-default law:
//! 1. in executable head position, every admitted surface lowers exactly once
//!    to its one-byte Function8;
//! 2. outside executable head position, a human spelling is not Function8
//!    identity and the evaluator must not recreate Function8 from that spelling;
//! 3. a user/global binding that happens to reuse a surface spelling cannot
//!    retarget an executable call, because lowering has already fixed the
//!    Function8 before evaluation.
//!
//! Source-migration coverage is owned by the role-aware active-SENS inventory;
//! this file deliberately does not re-scan contracts/evidence/generated data
//! as though every parenthesized datum were executable code.

use sens::{eval_program, load_core_library, lower_program, parse, ExprKind, Session, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// (code, namespace, spelling) from the generated registry used by lowering.
fn registry() -> Vec<(String, String, String)> {
    let text = fs::read_to_string(
        repo_root().join("crates/sens/src/semantic_registry_generated.rs"),
    )
    .unwrap();
    let mut rows = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.split("semantic_id: 0b").nth(1) else {
            continue;
        };
        let code: String = rest
            .chars()
            .take_while(|c| *c == '0' || *c == '1')
            .collect();
        for surface in rest
            .split("SemanticSurface { namespace: \"")
            .skip(1)
        {
            let mut parts = surface.splitn(2, "\", name: \"");
            let namespace = parts.next().unwrap_or("").to_owned();
            let name: String = parts
                .next()
                .unwrap_or("")
                .chars()
                .take_while(|c| *c != '"')
                .collect();
            if !name.is_empty() {
                rows.push((code.clone(), namespace, name));
            }
        }
    }
    rows
}

fn report(title: &str, offenders: &[String]) {
    assert!(
        offenders.is_empty(),
        "{title}: {}\n{}",
        offenders.len(),
        offenders
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_registry_surface_lowers_to_its_one_byte_sens_function() {
    let mut offenders = Vec::new();

    for (code, namespace, name) in registry() {
        let source = format!("({name})");
        match parse(&source) {
            Ok(parsed) => {
                let lowered = lower_program(&parsed);
                match &lowered[0].kind {
                    ExprKind::Call(sid, _) if sid.to_string() == code => {}
                    other => offenders.push(format!(
                        "{code} {namespace} {name}: lowered to {other:?}"
                    )),
                }
            }
            Err(error) => offenders.push(format!(
                "{code} {namespace} {name}: parse failed: {error}"
            )),
        }
    }

    report(
        "admitted surfaces that do not lower to their Function8",
        &offenders,
    );
}

#[test]
fn bare_human_surface_is_not_function8_identity_after_lowering() {
    let representatives = ["+", "atom?", "додати", "викликати"];

    for source in representatives {
        let parsed = parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
        let lowered = lower_program(&parsed);
        assert_eq!(lowered.len(), 1, "{source}");
        assert!(
            !matches!(lowered[0].kind, ExprKind::Sid(_)),
            "{source}: bare human spelling must not materialize Function8"
        );
    }
}

#[test]
fn a_surface_named_binding_cannot_retarget_an_executable_call() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core loads");

    // A human/global binding is a separate transition role. Even if the host
    // currently permits it, executable (+ ...) is lowered to 00001100 before
    // environment lookup and therefore cannot be redirected to this binding.
    let _ = eval_program("(def + 1)", &mut session);

    let result = eval_program("(+ 1 2)", &mut session)
        .expect("surface call must still lower to exact plus Function8");
    assert_eq!(
        result.value,
        Value::Number(3.0, sens::Exactness::Exact),
        "a binding named '+' must not retarget executable Function8 00001100"
    );

    let lowered = lower_program(&parse("(+ 1 2)").expect("parse plus call"));
    let ExprKind::Call(sid, _) = &lowered[0].kind else {
        panic!("plus call did not lower to Function8");
    };
    assert_eq!(sid.to_string(), "00001100");
}
