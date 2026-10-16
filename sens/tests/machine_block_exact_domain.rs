use sens::{
    eval_parsed_expressions, eval_program, load_core_library,
    parse_mixed_exact_domain_machine_source, Expr, ExprKind, Session,
};

const SOURCE: &str = include_str!("../../../lib/machine/block-exact.lisp");
const PATH: &str = "lib/machine/block-exact.lisp";

fn parsed_source() -> Vec<Expr> {
    parse_mixed_exact_domain_machine_source(PATH, SOURCE)
        .expect("точнодоменний файл машинних блоків має читатися лише власним шляхом")
}

#[test]
fn six_machine_block_definitions_have_exact_d4_and_no_legacy_w8_heads() {
    let definitions = parsed_source();
    assert_eq!(definitions.len(), 6);
    assert!(!SOURCE.contains("(00001001 "));
    assert!(!SOURCE.contains("(00001000 "));
    assert!(!SOURCE.contains("(00000001 "));
    assert!(!SOURCE.contains("(00100111 "));
    assert!(!SOURCE.contains("(00101001 "));

    for definition in &definitions {
        let ExprKind::List(items) = &definition.kind else {
            panic!("машинне визначення має бути структурним списком")
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(
            &items[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0011
        ), "DEFINE має мати точну D4-координату");
        let ExprKind::List(lambda) = &items[2].kind else {
            panic!("значення має бути безпосередньою LAMBDA")
        };
        assert!(matches!(
            &lambda[0].kind,
            ExprKind::DomainIdentity(identity)
                if identity.width() == 4 && identity.packed_bits() == 0b0010
        ), "LAMBDA має мати точну D4-координату");
    }
}

#[test]
fn exact_machine_block_preserves_nine_historical_observations() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("потрібна Lisp-owned реалізація LIST/APPEND");
    eval_parsed_expressions(&parsed_source(), &mut session)
        .expect("визначення мають виконуватися з точними доменами");

    let cases = [
        ("(machine-block-empty)", "()"),
        ("(machine-block-one (quote (mov r1 r2)))", "((mov r1 r2))"),
        ("(machine-block-append (quote ((mov r1 r2) (branch L1))) (quote ret))", "((mov r1 r2) (branch L1) ret)"),
        ("(machine-block-concat (quote ((mov r1 r2) (branch L1))) (quote ((label L2))))", "((mov r1 r2) (branch L1) (label L2))"),
        ("(machine-block-forms (quote ((mov r1 r2) (branch L1))))", "((mov r1 r2) (branch L1))"),
        ("(machine-block (quote ((mov r1 r2) (branch L1))))", "((mov r1 r2) (branch L1))"),
        ("(machine-block-append (quote ()) (quote nop))", "(nop)"),
        ("(machine-block-concat (quote ()) (quote ((mov r1 r2))))", "((mov r1 r2))"),
        ("(machine-block-concat (quote ((mov r1 r2))) (quote ()))", "((mov r1 r2))"),
    ];
    for (program, expected) in cases {
        let actual = eval_program(program, &mut session)
            .unwrap_or_else(|error| panic!("{program}: {error}"))
            .value.to_string();
        assert_eq!(actual, expected, "{program}");
    }
}

#[test]
fn x86_atom_let_cutover_uses_exact_d4_lambda_applications() {
    let source = include_str!("../../../lib/machine/atoms/x86-64.lisp");
    assert!(!source.contains("(001000 "), "виконувана D6 LET більше не потрібна");
    let expressions = parse_mixed_exact_domain_machine_source(
        "lib/machine/atoms/x86-64.lisp",
        source,
    ).expect("основний x86 Lisp-файл має читатися за точною доменною шириною");

    fn check(expression: &Expr, lambda_applications: &mut usize) {
        let ExprKind::List(items) = &expression.kind else { return };
        if let Some(Expr { kind: ExprKind::List(operator), .. }) = items.first() {
            if matches!(
                operator.first().map(|e| &e.kind),
                Some(ExprKind::DomainIdentity(identity))
                    if identity.width() == 4 && identity.packed_bits() == 0b0010
            ) {
                assert_eq!(items.len(), 2, "LET мав лише одне ініціалізоване зв'язування");
                assert_eq!(operator.len(), 3, "кожна LAMBDA має одне тіло");
                let ExprKind::List(parameters) = &operator[1].kind else {
                    panic!("параметр LAMBDA має бути структурним списком")
                };
                assert_eq!(parameters.len(), 1, "кожний локальний binder є одномісним");
                assert!(matches!(&parameters[0].kind, ExprKind::Symbol(_)));
                *lambda_applications += 1;
            }
        }
        for item in items.iter() { check(item, lambda_applications); }
    }

    let mut converted = 0;
    for expression in &expressions { check(expression, &mut converted); }
    assert_eq!(converted, 28, "усі 28 доведених LET перетворено без випадкових пропусків");
}


#[test]
fn x86_operand_rejection_predicate_returns_exact_d1_for_all_shapes() {
    let source = include_str!("../../../lib/machine/operands/x86-64.lisp");
    let parsed = parse_mixed_exact_domain_machine_source(
        "lib/machine/operands/x86-64.lisp",
        source,
    ).expect("лише машинний читач має обробляти цей Lisp-код");
    let mut session = Session::default();
    load_core_library(&mut session).expect("канонічна базова бібліотека");
    eval_parsed_expressions(&parsed, &mut session)
        .expect("визначення машинних операндів належать Lisp");

    for (argument, expected) in [
        ("(quote ())", false),
        ("(quote (gpr64 rax))", false),
        ("(quote (rejected machine-operand gpr64 invalid))", true),
    ] {
        let program = format!("(x86-machine-rejected? {argument})");
        let result = eval_program(&program, &mut session)
            .unwrap_or_else(|error| panic!("{program}: {error}"));
        assert_eq!(
            result.value.as_predicate_bit(),
            Some(expected),
            "тестовий предикат має повертати лише D1 так/ні: {program}"
        );
    }
}


#[test]
fn machine_block_reader_is_path_bound_and_quote_data_stays_opaque() {
    assert!(parse_mixed_exact_domain_machine_source(
        "lib/machine/other/block-exact.lisp", SOURCE
    ).is_ok(), "машинний reader не наділяє семантикою сам basename");
    assert!(parse_mixed_exact_domain_machine_source(
        "lib/not-machine/block-exact.lisp", SOURCE
    ).is_err());
    let quoted = parse_mixed_exact_domain_machine_source(
        PATH,
        "(001 (00001001 старе (00001000 (x) x)))",
    ).expect("цитовані дані не можна перетворювати на виклики");
    let ExprKind::List(items) = &quoted[0].kind else {
        panic!("QUOTE має лишатися списком")
    };
    assert!(matches!(&items[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b001));
    let ExprKind::List(payload) = &items[1].kind else {
        panic!("цитований payload має лишатися списком")
    };
    assert!(!matches!(&payload[0].kind, ExprKind::DomainIdentity(_)),
        "історична голова в цитаті не стає D8");
}
