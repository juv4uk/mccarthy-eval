use sens::{
    eval_program, load_core_library, load_time_library, semantic_registry_export, ErrorKind,
    Session, Value,
};

fn time_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();
    load_time_library(&mut session).unwrap();
    session
}

#[test]
fn core_quotient_and_mod_return_numeric_values_for_nonzero_divisors() {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();

    for (source, expected) in [
        ("(largest-chunk 10 3 3 1)", "(6 . 2)"),
        ("(quotient 10 3)", "3"),
        ("(quotient 1 3)", "0"),
        ("(mod 10 3)", "1"),
        ("(mod 0 7)", "0"),
    ] {
        assert_eq!(
            eval_program(source, &mut session)
                .unwrap_or_else(|error| panic!("{source}: {error}"))
                .value
                .to_string(),
            expected,
            "{source}"
        );
    }
}

/// A raw host observation is a Rust primitive on its SENS code (#1477), not a
/// name bound in the environment.
fn assert_raw_host_primitive(surface: &str) {
    let sid = semantic_registry_export::semantic_id_for_admitted_surface(surface)
        .unwrap_or_else(|| panic!("{surface} must stay admitted"));
    assert_eq!(
        semantic_registry_export::function_role(sid),
        Some("primitive"),
        "{surface} is a raw host primitive on its code"
    );
}

#[test]
fn utc_now_returns_utc_calendar_with_nanosecond_field() {
    let mut session = time_session();
    let value = eval_program("(utc-now)", &mut session)
        .unwrap()
        .value
        .to_string();
    let fields: Vec<&str> = value.trim_matches(['(', ')']).split_whitespace().collect();
    assert_eq!(fields.len(), 8, "utc-now shape: {value}");
    assert_eq!(fields[0], "utc");
    let month: i64 = fields[2].parse().unwrap();
    let day: i64 = fields[3].parse().unwrap();
    let hour: i64 = fields[4].parse().unwrap();
    let minute: i64 = fields[5].parse().unwrap();
    let second: i64 = fields[6].parse().unwrap();
    let nanosecond: i64 = fields[7].parse().unwrap();
    assert!((1..=12).contains(&month));
    assert!((1..=31).contains(&day));
    assert!((0..24).contains(&hour));
    assert!((0..60).contains(&minute));
    assert!((0..60).contains(&second));
    assert!((0..1_000_000_000).contains(&nanosecond));
}

#[test]
fn utc_calendar_conversion_is_language_owned() {
    let mut session = time_session();

    assert_eq!(
        eval_program("(utc-from-unix 0 0)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "(utc 1970 1 1 0 0 0 0)"
    );
    assert_eq!(
        eval_program("(utc-from-unix 946684800 123456789)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "(utc 2000 1 1 0 0 0 123456789)"
    );
    assert_eq!(
        eval_program("(utc-from-unix 1709164800 0)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "(utc 2024 2 29 0 0 0 0)"
    );
}

#[test]
fn raw_unix_clock_observation_is_interpreted_by_lisp() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(unix-time-observation->utc (quote (unix-time 946684800 42)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(utc 2000 1 1 0 0 0 42)"
    );
    assert_eq!(
        eval_program(
            "(unix-time-observation->utc (quote (not-unix-time 946684800 42)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-unix-time-observation)"
    );
}

#[test]
fn utc_now_exists_only_after_language_time_layer_loads() {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();

    let utc_sid = semantic_registry_export::semantic_id_for_admitted_surface("utc-now")
        .expect("utc-now must remain admitted by sr/2");
    assert_eq!(
        session.environment.get("utc-now"),
        None,
        "Core4 must not install a lexical placeholder before the time layer"
    );
    assert_eq!(
        semantic_registry_export::semantic_id_for_admitted_surface("utc-now"),
        Some(utc_sid),
        "surface admission still resolves to the exact SENS function"
    );
    let error = eval_program("(utc-now)", &mut session)
        .expect_err("unadmitted SID must fail closed before time library loads");
    assert_eq!(error.kind, ErrorKind::Type);
    assert_raw_host_primitive("unix-time-now");

    load_time_library(&mut session).unwrap();

    assert!(matches!(
        session.environment.get("utc-now"),
        Some(Value::Closure(_))
    ));
    assert_raw_host_primitive("unix-time-now");

    let value = eval_program("(utc-now)", &mut session)
        .unwrap()
        .value
        .to_string();
    let fields: Vec<&str> = value.trim_matches(['(', ')']).split_whitespace().collect();
    assert_eq!(fields.len(), 8, "language-owned utc-now shape: {value}");
    assert_eq!(fields[0], "utc");
}

#[test]
fn ntp_field_interpretation_is_language_owned() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(internet-time-fields->observation \"clock.example\" 4 1 2208988800 0)",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(accepted \"clock.example\" 0 0)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-fields->observation \"clock.example\" 5 15 2208988801 2147483648)",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(accepted \"clock.example\" 1 500000000)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-fields->observation \"clock.example\" 3 1 2208988800 0)",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-response)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-fields->observation \"clock.example\" 4 0 2208988800 0)",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-response)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-fields->observation \"clock.example\" 4 1 2208988799 0)",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-epoch)"
    );
}

#[test]
fn raw_ntp_observation_boundary_is_interpreted_by_lisp() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(internet-time-raw->observation (quote (ntp-fields \"clock.example\" 4 1 2208988801 2147483648)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(accepted \"clock.example\" 1 500000000)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-raw->observation (quote (ntp-fields \"clock.example\" 3 1 2208988800 0)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-response)"
    );
    assert_eq!(
        eval_program(
            "(internet-time-raw->observation (quote (rejected receive-failed)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected receive-failed)"
    );
}

#[test]
fn internet_time_sync_is_language_owned_after_time_library_loads() {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();

    let internet_time_sid =
        semantic_registry_export::semantic_id_for_admitted_surface("internet-time-sync")
            .expect("internet-time-sync must remain admitted by sr/2");
    assert_eq!(
        session.environment.get("internet-time-sync"),
        None,
        "Core4 must not install a lexical placeholder before the time layer"
    );
    assert_eq!(
        semantic_registry_export::semantic_id_for_admitted_surface("internet-time-sync"),
        Some(internet_time_sid),
        "surface admission still resolves to the exact SENS function"
    );
    let error = eval_program(
        "(internet-time-sync \"clock.example\" 1)",
        &mut session,
    )
    .expect_err("unadmitted SID must fail closed before time library loads");
    assert_eq!(error.kind, ErrorKind::Type);
    assert_raw_host_primitive("ntp-query-raw");

    load_time_library(&mut session).unwrap();

    assert!(matches!(
        session.environment.get("internet-time-sync"),
        Some(Value::Closure(_))
    ));
    assert_raw_host_primitive("ntp-query-raw");
}

#[test]
fn internet_time_timestamp_interpretation_is_language_owned() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(internet-time-observation->utc (quote (accepted \"clock.example\" 946684800 42)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(accepted \"clock.example\" (utc 2000 1 1 0 0 0 42))"
    );
    assert_eq!(
        eval_program(
            "(internet-time-observation->utc (quote (rejected receive-failed)))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected receive-failed)"
    );
}

#[test]
fn mono_ms_is_derived_from_nanoseconds_in_lisp() {
    let mut session = time_session();

    assert_eq!(
        eval_program("(milliseconds-from-nanoseconds 0)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "0"
    );
    assert_eq!(
        eval_program("(milliseconds-from-nanoseconds 999999)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "0"
    );
    assert_eq!(
        eval_program("(milliseconds-from-nanoseconds 1000000)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "1"
    );
    assert_eq!(
        eval_program("(milliseconds-from-nanoseconds 1999999)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "1"
    );
}

#[test]
fn mono_ms_binding_is_owned_by_lisp_after_time_library_loads() {
    let session = time_session();

    assert!(matches!(
        session.environment.get("mono-ms"),
        Some(Value::Closure(_))
    ));
    assert_raw_host_primitive("mono-ns");
}

#[test]
fn timezone_declaration_policy_is_language_owned() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(timezone-declarations->observation \"Europe/Kyiv\" \"Etc/UTC\")",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(detected \"Europe/Kyiv\" TZ)"
    );
    assert_eq!(
        eval_program(
            "(timezone-declarations->observation (quote ()) \"Europe/Kyiv\")",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(detected \"Europe/Kyiv\" etc-timezone)"
    );
    assert_eq!(
        eval_program(
            "(timezone-declarations->observation \"\" \"Europe/Kyiv\")",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(detected \"Europe/Kyiv\" etc-timezone)"
    );
    assert_eq!(
        eval_program(
            "(timezone-declarations->observation (quote ()) (quote ()))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(unknown host-declaration-unavailable)"
    );
}

#[test]
fn timezone_detection_is_explicit_and_ntp_requires_host_string() {
    let mut session = time_session();
    let timezone = eval_program("(timezone-detect)", &mut session)
        .unwrap()
        .value
        .to_string();
    assert!(timezone.starts_with("(detected ") || timezone.starts_with("(unknown "));
    assert!(eval_program("(internet-time-sync 123 100)", &mut session).is_err());
    assert_eq!(
        eval_program("(timezone-config \"Europe/Kyiv\" 7200)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "(accepted (timezone \"Europe/Kyiv\" 7200))"
    );

    assert_eq!(
        eval_program("(timezone-config 123 7200)", &mut session)
            .unwrap()
            .value
            .to_string(),
        "(rejected invalid-name)"
    );
}
