use sens::{eval_program, load_core_library, load_time_library, Session};

fn time_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();
    load_time_library(&mut session).unwrap();
    session
}

#[test]
fn exact_d3_time_source_repairs_preexisting_w8_cons_arity_error() {
    let input = "(timezone-raw->observation (quote (timezone-declarations \"Europe/Kyiv\" \"Etc/UTC\")))";
    fn observe(input: &str, migrated: bool) -> Result<String, String> {
        let mut session = Session::default();
        load_core_library(&mut session).map_err(|e| e.to_string())?;
        if migrated {
            load_time_library(&mut session).map_err(|e| e.to_string())?;
        } else {
            eval_program(include_str!("../../../tests/fixtures/time-d3-before.lisp"), &mut session)
                .map_err(|e| e.to_string())?;
        }
        eval_program(input, &mut session)
            .map(|x| x.value.to_string())
            .map_err(|e| e.to_string())
    }

    // Preserve the frozen pre-migration W8 error as evidence; never rewrite the fixture
    // to pretend this pre-existing arity defect was absent.
    let baseline = observe(input, false);
    match baseline {
        Err(error) => assert!(
            error.contains("expected 2 arguments; received 3"),
            "frozen W8 witness changed for an unexpected reason: {error}"
        ),
        Ok(value) => panic!("frozen W8 source unexpectedly changed behavior: {value}"),
    }

    // The current Lisp-owned implementation must repair the arity defect and retain
    // the public observation shape without a host-side fallback.
    assert_eq!(
        observe(input, true),
        Ok("(detected \"Europe/Kyiv\" TZ)".to_string()),
        "current exact-D3 time source must construct the observation via binary D3 CONS"
    );
}

#[test]
fn raw_timezone_declarations_are_interpreted_by_lisp() {
    let mut session = time_session();

    assert_eq!(
        eval_program(
            "(timezone-raw->observation (quote (timezone-declarations \"Europe/Kyiv\" \"Etc/UTC\")))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(detected \"Europe/Kyiv\" TZ)"
    );

    assert_eq!(
        eval_program(
            "(timezone-raw->observation (quote (timezone-declarations () \"Europe/Kyiv\")))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(detected \"Europe/Kyiv\" etc-timezone)"
    );

    assert_eq!(
        eval_program(
            "(timezone-raw->observation (quote (timezone-declarations () ())))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(unknown host-declaration-unavailable)"
    );

    // Empty strings are absent declarations too; timezone precedence must
    // remain language-owned and D3:110 must stay exactly two-clause.
    for (source, expected) in [
        (
            "(timezone-raw->observation (quote (timezone-declarations \"\" \"Etc/UTC\")))",
            "(detected \"Etc/UTC\" etc-timezone)",
        ),
        (
            "(timezone-raw->observation (quote (timezone-declarations \"\" \"\")))",
            "(unknown host-declaration-unavailable)",
        ),
        (
            "(timezone-raw->observation (quote (timezone-declarations \"Europe/Kyiv\" \"\")))",
            "(detected \"Europe/Kyiv\" TZ)",
        ),
        (
            "(timezone-raw->observation (quote (timezone-declarations () \"\")))",
            "(unknown host-declaration-unavailable)",
        ),
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

    assert_eq!(
        eval_program(
            "(timezone-raw->observation (quote (not-timezone-declarations () ())))",
            &mut session,
        )
        .unwrap()
        .value
        .to_string(),
        "(rejected invalid-timezone-observation)"
    );
}
