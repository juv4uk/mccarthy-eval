use sens::{
    eval_parsed_expressions, eval_program, load_core_library,
    parse_mixed_exact_domain_machine_source, Expr, ExprKind, Session,
};

const ATOMS_PATH: &str = "lib/machine/atoms/x86-64.lisp";
const ATOMS: &str = include_str!("../../../lib/machine/atoms/x86-64.lisp");
const OPERANDS_PATH: &str = "lib/machine/operands/x86-64.lisp";
const OPERANDS: &str = include_str!("../../../lib/machine/operands/x86-64.lisp");

#[test]
fn all_28_x86_let_bindings_are_exact_one_argument_d4_lambda_calls() {
    assert!(!ATOMS.contains("(001000 "), "D6 LET must not reappear as an executable head");
    let expressions = parse_mixed_exact_domain_machine_source(ATOMS_PATH, ATOMS)
        .expect("точні D3/D4 координати в основному x86-файлі");
    assert_eq!(expressions.len(), 132, "набір верхніх форм x86 не можна скорочувати");

    fn inspect(node: &Expr, converted: &mut usize) {
        let ExprKind::List(items) = &node.kind else { return };
        if let Some(Expr { kind: ExprKind::List(head), .. }) = items.first() {
            if matches!(
                head.first().map(|v| &v.kind),
                Some(ExprKind::DomainIdentity(identity))
                    if identity.width() == 4 && identity.packed_bits() == 0b0010
            ) {
                assert_eq!(items.len(), 2, "одне ініціалізоване зв'язування, як у старому LET");
                assert_eq!(head.len(), 3, "одна LAMBDA із єдиним тілом");
                let ExprKind::List(parameters) = &head[1].kind else {
                    panic!("параметри мають бути списком")
                };
                assert_eq!(parameters.len(), 1);
                assert!(matches!(&parameters[0].kind, ExprKind::Symbol(_)));
                *converted += 1;
            }
        }
        for child in items.iter() { inspect(child, converted); }
    }

    let mut migrated = 0;
    for expression in &expressions { inspect(expression, &mut migrated); }
    assert_eq!(migrated, 28, "кожне старе LET має мати рівно одну нову D4-форму");
}

#[test]
fn machine_rejection_returns_exact_d1_yes_or_no_instead_of_legacy_nil() {
    let forms = parse_mixed_exact_domain_machine_source(OPERANDS_PATH, OPERANDS)
        .expect("точні координати в бібліотеці операндів");
    let mut session = Session::default();
    load_core_library(&mut session).expect("міграційний базовий Lisp");
    eval_parsed_expressions(&forms, &mut session)
        .expect("визначення Lisp-owned операндів");

    for (argument, expected) in [
        ("(quote ())", false),
        ("(quote (gpr64 rax))", false),
        ("(quote (rejected machine-operand gpr64 invalid))", true),
    ] {
        let query = format!("(x86-machine-rejected? {argument})");
        let answer = eval_program(&query, &mut session)
            .unwrap_or_else(|error| panic!("{query}: {error}"));
        assert_eq!(
            answer.value.as_predicate_bit(),
            Some(expected),
            "предикат відхилення має давати точний D1: {query}"
        );
    }
}


#[test]
fn xmm_sentinel_shortens_quoted_data_without_collapsing_d3_and_d8() {
    // Функція x86-xmm-name? не викликає D8:00000000 CODE-CHAR:
    // її три цитовані singleton-списки потрібні лише як D3 ATOM = D1:NO.
    let marker_old = "(010 (001 (00000000)))";
    let marker_d3 = "(010 (001 (000)))";
    assert_eq!(OPERANDS.matches(marker_old).count(), 0);
    assert_eq!(OPERANDS.matches(marker_d3).count(), 3);
    assert_eq!(marker_old.len() - marker_d3.len(), 5);

    // Старий та новий цитовані payload різні: переносити CODE-CHAR у D3
    // за однакового числового значення заборонено. Доведено лише одну
    // спостережну властивість: непорожній список не є атомом.
    for (source, expected) in [
        (marker_old, false),
        (marker_d3, false),
        ("(010 (001 ()))", true),
    ] {
        let forms = parse_mixed_exact_domain_machine_source(OPERANDS_PATH, source)
            .unwrap_or_else(|error| panic!("точна граматика {source}: {error}"));
        let mut session = Session::default();
        let answer = eval_parsed_expressions(&forms, &mut session)
            .unwrap_or_else(|error| panic!("D3 ATOM {source}: {error}"));
        assert_eq!(
            answer.value.as_predicate_bit(),
            Some(expected),
            "D1 результат ATOM для цитованої структури: {source}"
        );
    }
}

#[test]
fn x86_lowerer_has_no_legacy_let() {
    let s = include_str!("../../../lib/machine/lowering/semantic-x86-64.lisp");
    assert!(!s.contains("(001000 "));
    assert!(!s.contains("(let ("));
}

#[test]
fn x86_lowerer_seven_d4_local_apps_and_62_opaque_w8_rows() {
    let source = include_str!("../../../lib/machine/lowering/semantic-x86-64.lisp");
    let ast = parse_mixed_exact_domain_machine_source(
        "lib/machine/lowering/semantic-x86-64.lisp", source
    ).expect("точнодоменний x86");
    assert_eq!(ast.len(), 37);
    // Шість мігрованих LET + одна раніше ратифікована локальна LAMBDA.
    assert_eq!(source.matches("((0010 (forms)").count(), 7);
    let history = source.split_once("; First executable semantic-lowering witness.").unwrap().0;
    let rows = history.lines().filter(|line| {
        let b = line.trim().trim_start_matches('(').as_bytes();
        b.len() > 9 && b[8] == b' '
            && b[..8].iter().all(|v| *v == b'0' || *v == b'1')
    }).count();
    assert_eq!(rows, 62, "цитовані W8 не є D8-викликами");
}

#[test]
fn x86_exact_width_witness_checks_2046_coordinates_and_18434_bits() {
    use sens::domain_ladder::DomainCoordinate;
    let (mut count, mut flips) = (0usize, 0usize);
    for width in 1..=10u8 {
        for bits in 0..(1u16 << width) {
            let c = DomainCoordinate::new(width, bits).unwrap();
            let spelling = c.видимі_біти();
            assert_eq!(spelling.len(), width as usize);
            assert_eq!(DomainCoordinate::з_видимих_бітів(&spelling), Some(c));
            count += 1;
            for i in 0..width as usize {
                let mut mutant = spelling.as_bytes().to_vec();
                mutant[i] = if mutant[i] == b'0' { b'1' } else { b'0' };
                let changed = DomainCoordinate::з_видимих_бітів(
                    std::str::from_utf8(&mutant).unwrap()
                ).unwrap();
                assert_eq!(changed.width(), width);
                assert_ne!(changed, c, "D{width}:{spelling}, bit {i}");
                flips += 1;
            }
        }
    }
    assert_eq!(count, 2046);
    assert_eq!(flips, 18_434);
}
