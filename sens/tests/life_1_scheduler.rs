//! Тільки механічний провідник: усі позитивні й негативні закони — у SENS.
//! ЖОДНИХ semantic expected cases або host-generated форм тут немає.

use sens::{eval_parsed_expressions, load_core_library, parse_mixed_exact_domain, Session};

#[test]
fn виконати_двійковий_свідок_life_1() {
    let mut session = Session::default();
    load_core_library(&mut session).expect("транспорт: завантажити SENS Core");

    let scheduler = include_str!("../../../lib/life-1-scheduler.lisp");
    let scheduler = parse_mixed_exact_domain(scheduler)
        .expect("транспорт: розібрати точні D3/D4 функції LIFE-1");
    eval_parsed_expressions(&scheduler, &mut session)
        .expect("транспорт: завантажити LIFE-1");

    // Єдиний оракул семантики: сама двійкова SENS-програма.
    let witness = include_str!("../../../tests/fixtures/life-1-exact-d3-semantic.lisp");
    let witness = parse_mixed_exact_domain(witness)
        .expect("транспорт: розібрати точний SENS-свідок");
    let result = eval_parsed_expressions(&witness, &mut session)
        .expect("транспорт: виконати SENS-свідок");

    // D1:NO is a real SENS failure. Ask SENS to expose its own 16 case bits;
    // Rust only prints the already-authored verdicts and NEVER changes truth.
    if result.value.to_string() != "1" {
        let probe = parse_mixed_exact_domain("(життя--свідок-біти)")
            .expect("транспорт: SENS-діагностичний виклик");
        let bits = eval_parsed_expressions(&probe, &mut session)
            .expect("транспорт: виконати SENS-діагностику");
        eprintln!("SENS_LIFE_1_CASE_BITS={}", bits.value);
    }

    // Це тільки спостереження вердикту D1:1, не перевірка логіки в Rust.
    assert_eq!(result.value.to_string(), "1", "SENS повернув не D1:YES");
}
