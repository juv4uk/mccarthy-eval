use sens::{
    eval_program,
    semantic_registry_export::admitted_surfaces_for_semantic_id,
    ErrorKind, Sens8, Session, Value,
};

fn eval_value(session: &mut Session, source: &str) -> Value {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
}

#[test]
fn exact_function_carries_itself_across_two_language_stages() {
    let mut session = Session::default();
    assert_eq!(
        session.environment.selected_core_profile(),
        None,
        "свідок не повинен залежати від завантажувача профілю Core"
    );

    // Стадія A — exact-SENS тотожне замикання. Жодне поверхневе ім'я функції
    // не бере участі в перенесенні: на вхід і вихід проходить сама функція
    // 00000101.
    let carried = eval_value(&mut session, "((00001000 (f) f) 00000101)");
    assert_eq!(carried, Value::Sid(sens::sens!(00000101)));

    // Стадія B отримує результат стадії A як виконувану голову. Якби стадія A
    // реконструювала ім'я/рядок/число замість перенесення самої функції,
    // цей виклик не дійшов би до exact-механізму 00000101.
    let result = eval_value(
        &mut session,
        "(((00001000 (f) f) 00000101) (00000001 (alpha beta)))",
    );
    assert_eq!(result, Value::Symbol("alpha".into()));

    assert_eq!(
        session.environment.selected_core_profile(),
        None,
        "виконання перенесеної exact-функції не повинно неявно вибирати Core"
    );
}

fn poison_surfaces_for(session: &mut Session, function: Sens8) -> usize {
    let surfaces = admitted_surfaces_for_semantic_id(function);
    for surface in &surfaces {
        session
            .environment
            .define(surface.name, Value::Symbol("surface-poison".into()));
    }
    surfaces.len()
}

#[test]
fn exact_path_ignores_poisoned_surface_bindings() {
    let mut session = Session::default();

    // Це adversarial setup, а не transport: exact-функціями знаходимо лише
    // їхні людські peer-surfaces і робимо ці bindings явно непридатними.
    let poisoned = [
        sens::sens!(00000001),
        sens::sens!(00000101),
        sens::sens!(00001000),
    ]
    .into_iter()
    .map(|function| poison_surfaces_for(&mut session, function))
    .sum::<usize>();
    assert!(poisoned > 0, "adversarial setup має реально отруїти surface bindings");
    assert_eq!(
        session.environment.get("car"),
        Some(Value::Symbol("surface-poison".into())),
        "контроль має довести, що peer surface 00000101 справді отруєний"
    );

    let carried = eval_value(&mut session, "((00001000 (f) f) 00000101)");
    assert_eq!(carried, Value::Sid(sens::sens!(00000101)));

    let result = eval_value(
        &mut session,
        "(((00001000 (f) f) 00000101) (00000001 (alpha beta)))",
    );
    assert_eq!(result, Value::Symbol("alpha".into()));
}

#[test]
fn unsupported_exact_function_is_still_carried_as_the_same_function() {
    let mut session = Session::default();

    let carried = eval_value(&mut session, "((00001000 (f) f) 11111111)");
    assert_eq!(carried, Value::Sid(sens::sens!(11111111)));

    let error = eval_program(
        "(((00001000 (f) f) 11111111))",
        &mut session,
    )
    .expect_err("exact-функція має впасти лише на admission виконуваного механізму");

    assert_eq!(error.kind, ErrorKind::Type);
    assert_eq!(
        error.message,
        "SENS function has no admitted callable mechanism: 11111111"
    );
}
