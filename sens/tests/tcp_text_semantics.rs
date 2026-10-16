use sens::{eval_program, load_core_library, load_tcp_library, Session, Value};

fn tcp_semantics_session() -> Session {
    let mut session = Session::default();
    load_core_library(&mut session).unwrap();
    load_tcp_library(&mut session).unwrap();
    session
}

#[test]
fn tcp_text_interpretation_is_language_owned() {
    let mut session = tcp_semantics_session();

    let ascii = eval_program("(tcp-read-bytes->text (quote (104 105)))", &mut session)
        .expect("ASCII should decode in Lisp");
    assert_eq!(ascii.value, Value::String("hi".into()));

    let cyrillic = eval_program(
        "(tcp-read-bytes->text (quote (208 159 209 128 208 184 208 178 209 150 209 130)))",
        &mut session,
    )
    .expect("UTF-8 should decode in Lisp");
    assert_eq!(cyrillic.value, Value::String("Привіт".into()));

    let invalid = eval_program("(tcp-read-bytes->text (quote (255)))", &mut session)
        .expect("invalid UTF-8 should become explicit language data");
    assert_eq!(invalid.value.to_string(), "(rejected invalid-utf8)");

    let eof = eval_program("(tcp-read-bytes->text (quote ()))", &mut session)
        .expect("EOF should preserve the historical empty-string public meaning");
    assert_eq!(eof.value, Value::String("".into()));
}
