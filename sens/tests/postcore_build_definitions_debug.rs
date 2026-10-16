use sens::{eval_program, load_core_library, Session};

#[test]
fn print_postcore_definition_data_shape() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core should load");
    let result = eval_program(
        "(write-to-string (my-postcore-build-definitions (quote process-run) (quote (запустити-процес))))",
        &mut session,
    )
    .expect("core helper should evaluate");
    println!("POSTCORE_BUILD_DEFINITIONS = {}", result.value);
}
