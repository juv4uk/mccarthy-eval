use sens::{eval_program, load_core_library, Session};

#[test]
fn migrated_user_demos_execute_with_current_domain_surfaces() {
    for (name, source) in [
        ("task-demo.lisp", include_str!("../../../task-demo.lisp")),
        ("demo-tasks.lisp", include_str!("../../../demo-tasks.lisp")),
        ("tasks-summary.lisp", include_str!("../../../tasks-summary.lisp")),
        ("tasks-summary2.lisp", include_str!("../../../tasks-summary2.lisp")),
        ("task-stats.lisp", include_str!("../../../task-stats.lisp")),
    ] {
        let mut session = Session::default();
        load_core_library(&mut session).expect("Core bootstrap must be available");
        let result = eval_program(source, &mut session)
            .unwrap_or_else(|error| panic!("{name}: migrated source failed: {error:?}"));
        assert_eq!(result.value.to_string(), "225", "unexpected result in {name}");
    }
}

#[test]
fn migrated_task_info_arithmetic_preserves_exact_observation_without_host_io() {
    let source = include_str!("../../../tasks-info.lisp");
    // Перевіряємо тільки змінений арифметичний вираз; host-вивід не мігруємо.
    let source_arithmetic = source.split("(format").next()
        .expect("source contains arithmetic prelude");
    let mut session = Session::default();
    load_core_library(&mut session).expect("Core bootstrap");
    let result = eval_program(source_arithmetic, &mut session)
        .expect("domain-typed arithmetic prelude must evaluate");
    assert_eq!(result.value.to_string(), "225");
}
