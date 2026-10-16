use sens::{
    eval_parsed_expressions, parse_mixed_exact_domain_machine_source,
    ExprKind, Session,
};

const PATH: &str = "lib/machine/admission/x86-64.lisp";
const SOURCE: &str = include_str!("../../../lib/machine/admission/x86-64.lisp");

fn predicate(source: &str) -> bool {
    let forms = parse_mixed_exact_domain_machine_source(PATH, source)
        .expect("джерело машинного допуску має точні D3-голови");
    let mut session = Session::default();
    let result = eval_parsed_expressions(&forms, &mut session)
        .expect("структурний ATOM має відповідати ратифікованому D1");
    result.value.as_predicate_bit()
        .expect("D3 ATOM зобов'язаний повертати лише D1 так або ні")
}

#[test]
fn shorter_quoted_singleton_preserves_observable_d1_no() {
    let original = "(010 (001 (00000000)))";
    let current = "(010 (001 (000)))";
    assert!(!predicate(original), "історичний цитований одноелементний список не атом");
    assert_eq!(predicate(original), predicate(current), "заміна не може змінити D1 відповідь");
    assert!(!predicate(current));

    // Якщо видалити саме елемент, буде порожній список — це інша семантика.
    assert!(predicate("(010 (001 ()))"), "негативний свідок: ATOM від порожнього списку є D1 так");
}

#[test]
fn admission_source_has_exactly_27_short_inert_payloads_and_no_old_d8_sentinel() {
    assert_eq!(SOURCE.matches("(010 (001 (000)))").count(), 27);
    assert!(!SOURCE.contains("(00000000)"),
        "історична восьмибітна позначка не повинна повертатися");
    assert!(SOURCE.contains("(11110100 value)"),
        "справжня окрема D8-функція не повинна змінитися");

    let expressions = parse_mixed_exact_domain_machine_source(PATH, SOURCE)
        .expect("увесь admission-файл має проходити суворий машинний reader");
    assert_eq!(expressions.len(), 18, "зміни не можуть вилучати визначення машинного допуску");
    for definition in &expressions {
        let ExprKind::List(list) = &definition.kind else {
            panic!("верхні визначення повинні лишатися списками")
        };
        assert!(list.len() >= 3, "верхнє визначення повинно лишатися цілісним");
    }
}

#[test]
fn quote_payload_remains_data_and_cannot_be_relabelled_as_callable_d8() {
    let form = parse_mixed_exact_domain_machine_source(
        PATH, "(010 (001 (000)))",
    ).expect("читач із точними ширинами");
    let ExprKind::List(atom) = &form[0].kind else { panic!("ATOM") };
    let ExprKind::List(quoted) = &atom[1].kind else { panic!("QUOTE") };
    assert!(matches!(&quoted[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b001));
    let ExprKind::List(singleton) = &quoted[1].kind else { panic!("цитовані дані") };
    assert_eq!(singleton.len(), 1);
    assert!(!matches!(&singleton[0].kind, ExprKind::DomainIdentity(_)),
        "цитований бінарний datum не набуває права виконання D3/D8");
}
