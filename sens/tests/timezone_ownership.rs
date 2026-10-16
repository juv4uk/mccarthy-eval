use sens::{
    eval_program, load_core_library, load_time_library, semantic_registry_export, ErrorKind,
    Session, Value,
};

#[test]
fn timezone_detect_public_binding_is_language_owned_over_raw_declarations() {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();

    let timezone_sid =
        semantic_registry_export::semantic_id_for_admitted_surface("timezone-detect")
            .expect("timezone-detect must remain admitted by sr/2");
    assert_eq!(
        session.environment.get("timezone-detect"),
        None,
        "Core4 must not install a lexical placeholder before the time layer"
    );
    assert_eq!(
        semantic_registry_export::semantic_id_for_admitted_surface("timezone-detect"),
        Some(timezone_sid),
        "surface admission still resolves to the exact SENS function"
    );
    let error = eval_program("(timezone-detect)", &mut session)
        .expect_err("unadmitted SID must fail closed before time library loads");
    assert_eq!(error.kind, ErrorKind::Type);
    assert!(session.environment.get("timezone-detect-raw").is_none());
    // Після #1477 — примітив за кодом СЕНС, не прив'язка Value::Builtin.
    assert_eq!(
        sens::semantic_registry_export::semantic_id_for_admitted_surface("timezone-declarations-raw")
            .and_then(sens::semantic_registry_export::function_role),
        Some("primitive")
    );

    load_time_library(&mut session).unwrap();

    assert!(matches!(
        session.environment.get("timezone-detect"),
        Some(Value::Closure(_))
    ));
    assert!(session.environment.get("timezone-detect-raw").is_none());
    // Після #1477 — примітив за кодом СЕНС, не прив'язка Value::Builtin.
    assert_eq!(
        sens::semantic_registry_export::semantic_id_for_admitted_surface("timezone-declarations-raw")
            .and_then(sens::semantic_registry_export::function_role),
        Some("primitive")
    );
}
