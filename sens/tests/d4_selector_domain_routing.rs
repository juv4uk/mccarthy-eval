use sens::{eval_program, load_core_library, Session};

fn eval(source: &str) -> String {
    let mut session = Session::default();
    load_core_library(&mut session).expect("core library");
    eval_program(source, &mut session)
        .unwrap_or_else(|error| panic!("{source}: {error}"))
        .value
        .to_string()
}

#[test]
fn existing_selector_surfaces_execute_after_d4_domain_cutover() {
    assert_eq!(eval("(caar '((11 12) 13))"), "11");
    assert_eq!(eval("(cadr '(11 12 13))"), "12");
    assert_eq!(eval("(cddr '(11 12 13))"), "(13)");
}


#[test]
fn historical_selector_bytes_delegate_to_domain_slots_without_dual_binding() {
    assert_eq!(eval("(00110011 '((11 12) 13))"), "11");
    assert_eq!(eval("(00110100 '(11 12 13))"), "12");
    assert_eq!(eval("(00110101 '(11 12 13))"), "(13)");
}
