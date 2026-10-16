use sens::{eval_program, load_core_library, ErrorKind, Session};

fn loaded_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library should load");
    session
}

fn value(source: &str, session: &mut Session) -> String {
    eval_program(source, session)
        .unwrap_or_else(|error| panic!("{source}: {error:?}"))
        .value
        .to_string()
}

#[test]
fn exact_let_code_matches_surface_semantics() {
    let mut surface = loaded_session();
    let mut exact = loaded_session();

    assert_eq!(
        value("(let ((x 1) (y 2)) (+ x y))", &mut surface),
        value("(10011100 ((x 1) (y 2)) (00001100 x y))", &mut exact)
    );
    assert_eq!(
        value("(let () 42)", &mut surface),
        value("(10011100 () 42)", &mut exact)
    );

    let surface_error =
        eval_program("(let ((x 1) (y x)) (+ x y))", &mut surface).unwrap_err();
    let exact_error =
        eval_program("(10011100 ((x 1) (y x)) (00001100 x y))", &mut exact)
            .unwrap_err();
    assert_eq!(surface_error.kind, ErrorKind::UnknownSymbol);
    assert_eq!(exact_error.kind, ErrorKind::UnknownSymbol);
}

#[test]
fn exact_let_star_code_matches_surface_semantics() {
    let mut surface = loaded_session();
    let mut exact = loaded_session();

    let surface_program =
        "(let* ((x 1) (y (+ x 1)) (z (+ y 1))) (list x y z))";
    let exact_program =
        "(10011101 ((x 1) (y (00001100 x 1)) (z (00001100 y 1))) (00100111 x y z))";

    assert_eq!(value(surface_program, &mut surface), "(1 2 3)");
    assert_eq!(value(exact_program, &mut exact), "(1 2 3)");
    assert_eq!(
        value("(let* () 7)", &mut surface),
        value("(10011101 () 7)", &mut exact)
    );
}
