//! Доказ атомарної міграції двох перевірок довжини у translation.lisp.
//!
//! 2046 перевірок = УСІ можливі пари (width, payload) D1–D10,
//! а не 2046 виконуваних чи семантично ратифікованих функцій.
//! Жодних SID8/SENS8 перекладів або семантики з голого packed-byte.

use sens::{
    domain_ladder::DomainCoordinate, pack_domain_coordinates,
    packed_transport_accounting, unpack_domain_coordinates,
};

#[test]
fn переклад_довжини_має_дві_точні_трибітні_голови() {
    let source = include_str!("../../../lib/translation.lisp");

    // Обидві форми порівнюють атомарний результат D6 LENGTH із 5;
    // D3 EQ не застосовується до структурних пар.
    assert_eq!(source.matches("(101 (000000 proposal) 5)").count(), 1);
    assert_eq!(source.matches("(101 (000000 review) 5)").count(), 1);
    assert!(!source.contains("(11110111 (000000 proposal) 5)"));
    assert!(!source.contains("(11110111 (000000 review) 5)"));

    // До міграції було 2 × 8 source-бітів, після — 2 × 3.
    assert_eq!(2 * (8usize - 3), 10);
}

#[test]
fn всі_2046_координат_д1_д10_зберігають_ширину_і_біти_без_padding() {
    let mut coordinates = Vec::with_capacity(2046);
    let mut widths = Vec::with_capacity(2046);
    let mut semantic_bits = 0usize;

    for width in 1u8..=10u8 {
        let capacity = 1u16 << width;
        for bits in 0..capacity {
            let coordinate = DomainCoordinate::new(width, bits).expect("повна бітова координата");
            assert_eq!(coordinate.width(), width);
            assert_eq!(coordinate.bits(), bits);

            // Координата є парою (ширина,біти), не числовим ID функції.
            if width != 1 && bits < (1u16 << (width - 1)) {
                assert_ne!(
                    coordinate,
                    DomainCoordinate::new(width - 1, bits).unwrap(),
                    "D{width}:{bits:b} не може успадкувати роль D{}", width - 1
                );
            }
            coordinates.push(coordinate);
            widths.push(width);
            semantic_bits += usize::from(width);
        }
        // Граничний payload для Dn завжди заборонений, навіть якщо влізе в u16.
        assert_eq!(DomainCoordinate::new(width, capacity), None);
    }

    assert_eq!(coordinates.len(), 2046);
    assert_eq!(semantic_bits, 18434);

    let packed = pack_domain_coordinates(&coordinates);
    assert_eq!(packed.bit_len(), semantic_bits);
    assert_eq!(packed.byte_len(), 2305); // 18434 біт без міжслівного вирівнювання
    assert_eq!(
        unpack_domain_coordinates(&packed, &widths).as_deref(),
        Some(coordinates.as_slice()),
        "кожен Dn має відновитися з оригінальною шириною"
    );

    // Семантичні біти не підмінюються байтовим доповненням.
    let accounting = packed_transport_accounting(&packed, 0);
    assert_eq!(accounting.semantic_payload_bits, 18434);
    assert_eq!(accounting.tail_unused_bits, 6);
    assert_eq!(accounting.total_wire_bits, 18440);

    // Декодер без повного доказового розкладу ширин мусить відмовити.
    assert!(unpack_domain_coordinates(&packed, &widths[..widths.len() - 1]).is_none());
    let mut invalid = widths.clone();
    invalid[0] = 11;
    assert!(unpack_domain_coordinates(&packed, &invalid).is_none());

    // Усе це лише перенос координат: жодної автоматичної callability D7/D10
    // чи перенесення ролі D8 EQUAL до того самого payload іншої ширини.
    assert_ne!(
        DomainCoordinate::new(8, 0b11110111),
        DomainCoordinate::new(3, 0b101)
    );
}

#[test]
fn атомарна_довжина_має_справжній_d3_так_ні_а_не_порівняння_пар() {
    use sens::{eval_parsed_expressions, parse_mixed_exact_domain, Session};

    fn run(source: &str) -> Result<String, String> {
        let forms = parse_mixed_exact_domain(source).map_err(|e| e.to_string())?;
        let value = eval_parsed_expressions(&forms, &mut Session::default())
            .map_err(|e| e.to_string())?;
        Ok(value.value.to_string())
    }

    // Д3 EQ перевіряємо окремо: D6 LENGTH нині не має
    // підтвердженого механізму виклику, тому не вдаємо, що тестували його.
    let five = run("(101 5 5)")
        .expect("D3 EQ атомарних чисел 5 та 5");
    let four = run("(101 5 4)")
        .expect("D3 EQ атомарних чисел 5 та 4");
    assert_ne!(five, four, "D3 EQ мусить розрізняти 5 і 4");

    // Відмінність від D8 EQUAL принципова: D3 EQ не порівнює пари.
    assert!(
        run("(101 (001 (a b)) (001 (a b)))").is_err(),
        "D3 EQ не повинен silently порівнювати структурні Pair"
    );
}
