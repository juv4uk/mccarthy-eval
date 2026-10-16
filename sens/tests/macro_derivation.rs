use sens::{
    eval_program, load_macro_library, parse_mixed_exact_domain_macro_source, Environment, Expr,
    ExprKind, Session, Value, MACRO_LIBRARY_SOURCE,
};
use std::rc::Rc;

fn eval_with_derived_macros(source: &str) -> String {
    let mut session = Session::default();
    eval_program(source, &mut session)
        .expect("program using derived macros should evaluate")
        .value
        .to_string()
}

fn assert_same_macro_value(values: [Value; 2]) {
    match (&values[0], &values[1]) {
        (Value::Macro(defmacro), Value::Macro(uk)) => {
            assert!(
                Rc::ptr_eq(defmacro, uk),
                "defmacro and визначити-макрос must share one Macro value"
            );
        }
        other => panic!("expected two Macro bindings, got {other:?}"),
    }
}

fn binary_runs(source: &str) -> Vec<&str> {
    source
        .split(|ch: char| ch != '0' && ch != '1')
        .filter(|token| !token.is_empty())
        .collect()
}

fn scan_exact_macro_ast(
    expr: &Expr,
    legacy: &mut usize,
    d3: &mut Vec<u16>,
    d4: &mut Vec<u16>,
) {
    match &expr.kind {
        ExprKind::Sid(_) => *legacy += 1,
        ExprKind::DomainIdentity(identity) => match identity.width() {
            3 => d3.push(identity.packed_bits()),
            4 => d4.push(identity.packed_bits()),
            _ => {}
        },
        ExprKind::List(items) => {
            for item in items.iter() {
                scan_exact_macro_ast(item, legacy, d3, d4);
            }
        }
        _ => {}
    }
}

#[test]
fn migrated_macro_bootstrap_is_d3_d4_only_and_executes_generated_define_lambda() {
    let legacy_words = binary_runs(MACRO_LIBRARY_SOURCE)
        .into_iter()
        .filter(|word| word.len() == 8)
        .collect::<Vec<_>>();
    assert!(
        legacy_words.is_empty(),
        "lib/macro.lisp must not retain executable/data SID8 tokens: {legacy_words:?}"
    );

    let parsed = parse_mixed_exact_domain_macro_source("lib/macro.lisp", MACRO_LIBRARY_SOURCE)
        .expect("migrated macro source must use its exact-domain reader");
    let mut legacy = 0;
    let mut d3 = Vec::new();
    let mut d4 = Vec::new();
    for expr in &parsed {
        scan_exact_macro_ast(expr, &mut legacy, &mut d3, &mut d4);
    }
    assert_eq!(legacy, 0, "macro AST must contain no legacy SID nodes");
    for required in [0b001, 0b010, 0b011, 0b100, 0b110, 0b111] {
        assert!(d3.contains(&required), "missing D3:{required:03b}");
    }
    assert!(
        d4.iter().filter(|&&bits| bits == 0b0010).count() >= 2,
        "D4:0010 must exist both as the bootstrap lambda and generated code data"
    );
    assert!(d4.contains(&0b0011), "generated DEFINE must be exact D4:0011");

    let environment = Environment::root();
    let mut session = Session { environment };
    load_macro_library(&mut session).expect("exact-domain macro bootstrap must load");
    let result = eval_program(
        "(defmacro exact-identity (x) x) (exact-identity 37)",
        &mut session,
    )
    .expect("generated D4 DEFINE/LAMBDA must execute after value->expr roundtrip");
    assert_eq!(result.value.to_string(), "37");
}

#[test]
fn default_session_binds_all_defmacro_peers_to_one_value() {
    let session = Session::default();
    assert_same_macro_value([
        session
            .environment
            .get("defmacro")
            .expect("default session must bind defmacro"),
        session
            .environment
            .get("визначити-макрос")
            .expect("default session must bind визначити-макрос"),
    ]);
}

#[test]
fn macro_peer_admission_is_recorded_under_identity_00001010_without_binding_the_machine_id() {
    let id = sens::semantic_registry_export::semantic_id_for_admitted_surface("defmacro")
        .expect("defmacro must be registry-admitted");
    assert_eq!(
        sens::semantic_registry_export::semantic_id_bits(id),
        "00001010"
    );

    let surfaces = sens::semantic_registry_export::admitted_surfaces_for_semantic_id(id);
    assert!(surfaces.iter().any(|row| row.namespace == "en" && row.name == "defmacro"));
    assert!(surfaces.iter().any(|row| row.namespace == "ук" && row.name == "визначити-макрос"));
    assert!(surfaces.iter().any(|row| row.namespace == "укр" && row.name == "визначити-макрос"));
    assert!(surfaces.iter().all(|row| row.namespace != "sa" && row.namespace != "sym"));

    let session = Session::default();
    assert!(
        session.environment.get("00001010").is_none(),
        "opaque semantic IDs must never become ordinary lexical bindings"
    );
}
#[test]
fn bare_root_gains_peer_bindings_only_through_macro_loader() {
    let environment = Environment::root();
    assert!(environment.get("defmacro").is_none());
    assert!(environment.get("визначити-макрос").is_none());

    let mut session = Session { environment };
    let loaded = load_macro_library(&mut session).expect("macro library should bootstrap");
    let loaded_macro = match &loaded.value {
        Value::Macro(value) => value,
        other => panic!("macro library must return one Macro value, got {other:?}"),
    };

    let defmacro = session
        .environment
        .get("defmacro")
        .expect("loader must bind defmacro");
    let uk = session
        .environment
        .get("визначити-макрос")
        .expect("loader must bind визначити-макрос");

    assert_same_macro_value([defmacro.clone(), uk]);
    match &defmacro {
        Value::Macro(bound) => assert!(
            Rc::ptr_eq(loaded_macro, bound),
            "loader must bind the exact Macro value returned by lib/macro.lisp"
        ),
        other => panic!("defmacro binding must be a Macro value, got {other:?}"),
    }
}

#[test]
fn language_owned_defmacro_introduces_a_working_macro() {
    let value = eval_with_derived_macros(
        r#"
        (defmacro identity (x) x)
        (identity 42)
        "#,
    );
    assert_eq!(value, "42");
}

#[test]
fn ukrainian_defmacro_peer_works_without_loading_uk_surface_bridge() {
    let value = eval_with_derived_macros(
        r#"
        (визначити-макрос identity-uk (x) x)
        (identity-uk 11)
        "#,
    );
    assert_eq!(value, "11");
}

#[test]
fn language_owned_defmacro_preserves_unevaluated_arguments() {
    let value = eval_with_derived_macros(
        r#"
        (defmacro first-form (a b) a)
        (first-form (quote ok) never-defined)
        "#,
    );
    assert_eq!(value, "ok");
}

#[test]
fn language_owned_defmacro_can_build_control_flow() {
    let value = eval_with_derived_macros(
        r#"
        (defmacro unless (condition body)
          (cons (quote cond)
            (cons
              (cons condition
                (cons (quote ()) (quote ())))
              (cons
                (cons (quote t)
                  (cons body (quote ())))
                (quote ())))))
        (unless () (quote success))
        "#,
    );
    assert_eq!(value, "success");
}
