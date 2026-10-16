use sens::{eval_program, ErrorKind, Session};

fn eval(source: &str) -> String {
    eval_program(source, &mut Session::default())
        .unwrap()
        .value
        .to_string()
}

#[test]
fn tagged_literals_are_self_evaluating_and_canonical() {
    assert_eq!(eval("#i32(1 -2 3)"), "#i32(1 -2 3)");
    assert_eq!(eval("#f32(1.0 0.1 -2.5)"), "#f32(1.0 0.1 -2.5)");
}

#[test]
fn accessors_preserve_element_domain() {
    assert_eq!(eval("(numeric-buffer-type #f32())"), "f32");
    assert_eq!(eval("(numeric-buffer-length #i32(4 5 6))"), "3");
    assert_eq!(eval("(numeric-buffer-ref #i32(-7) 0)"), "-7");
    assert_eq!(eval("(numeric-buffer-ref #f32(3.0) 0)"), "3.0");
}

#[test]
fn conversion_failures_are_named_and_never_wrap() {
    let type_error = eval_program("(i32-buffer 1/2)", &mut Session::default()).unwrap_err();
    assert_eq!(type_error.kind, ErrorKind::Type);

    let overflow = eval_program("(i32-buffer 2147483648)", &mut Session::default()).unwrap_err();
    assert_eq!(overflow.kind, ErrorKind::NumericOverflow);

    let bounds =
        eval_program("(numeric-buffer-ref #i32(1) 1)", &mut Session::default()).unwrap_err();
    assert_eq!(bounds.kind, ErrorKind::InvalidForm);
}

#[test]
fn printed_buffers_round_trip_through_read() {
    assert_eq!(eval("(read \"#i32(1 -2 3)\")"), "#i32(1 -2 3)");
    assert_eq!(eval("(read \"#f32(1.0 0.1 -2.5)\")"), "#f32(1.0 0.1 -2.5)");
}

#[test]
fn canonical_buffer_map_preserves_type_and_checks_results() {
    assert_eq!(
        eval("(numeric-buffer-map (lambda (x) (+ x 1)) #i32(1 2 3))"),
        "#i32(2 3 4)"
    );
    assert_eq!(
        eval("(numeric-buffer-map (lambda (x) (+ x 0.5)) #f32(1.0 2.0))"),
        "#f32(1.5 2.5)"
    );

    let overflow = eval_program(
        "(numeric-buffer-map (lambda (x) (+ x 1)) #i32(2147483647))",
        &mut Session::default(),
    )
    .unwrap_err();
    assert_eq!(overflow.kind, ErrorKind::NumericOverflow);

    let wrong_result = eval_program(
        "(numeric-buffer-map (lambda (x) (quote nope)) #i32(1))",
        &mut Session::default(),
    )
    .unwrap_err();
    assert_eq!(wrong_result.kind, ErrorKind::Type);
}
