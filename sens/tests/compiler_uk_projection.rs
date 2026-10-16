//! Розділяємо ратифіковану українську проєкцію і історичний восьмибітний носій.
//! Старий W8 не можна підвищувати до сучасного D4 за схожістю написання.
use sens::{parse_mixed_exact_domain, ExprKind};

const ДЖЕРЕЛО: &str = include_str!("../../../lib/compiler-nucleus.lisp");

#[test]
fn історичні_51_визначення_не_стають_неявним_d4() {
    // Компільований носій був мігрований із українських поверхонь у 8-бітні
    // історичні голови. Зберігаємо доказ кількості й заборони W8 → D4.
    let кількість = ДЖЕРЕЛО.matches("(00001001").count();
    assert_eq!(кількість, 51, "потрібно зберегти всі 51 визначення");
    assert_eq!(ДЖЕРЕЛО.matches("(00001000").count(), кількість);

    let форми = parse_mixed_exact_domain(ДЖЕРЕЛО)
        .expect("історичний носій компілятора мусить розбиратися без переозначення");
    assert_eq!(форми.len(), кількість);

    for (номер, форма) in форми.iter().enumerate() {
        let ExprKind::List(визначення) = &форма.kind else {
            panic!("верхня форма {номер} не є списком");
        };
        assert_eq!(визначення.len(), 3, "арність визначення {номер}");
        assert_eq!(
            ДЖЕРЕЛО.get(визначення[0].span.start..визначення[0].span.end),
            Some("00001001"),
            "потрібно зберегти точну восьмибітну голову визначення {номер}"
        );
        assert!(
            !matches!(&визначення[0].kind, ExprKind::DomainIdentity(_)),
            "W8 визначення {номер} не може без доказу перетворитися на D4"
        );
        assert!(
            !matches!(&визначення[1].kind, ExprKind::DomainIdentity(_)),
            "ім'я визначення {номер} — дані зв'язування"
        );

        let ExprKind::List(лямбда) = &визначення[2].kind else {
            panic!("значення визначення {номер} має бути лямбдою");
        };
        assert!(лямбда.len() >= 3, "лямбда {номер} потребує тіла");
        assert_eq!(
            ДЖЕРЕЛО.get(лямбда[0].span.start..лямбда[0].span.end),
            Some("00001000"),
            "точна восьмибітна голова лямбди {номер}"
        );
        assert!(
            !matches!(&лямбда[0].kind, ExprKind::DomainIdentity(_)),
            "історичний W8 LAMBDA не є ратифікованим D4"
        );
    }
}

#[test]
fn український_приклад_піднімає_лише_ратифіковані_голови() {
    // Це тільки читання української поверхні, НЕ доказ фізичного .sens.
    // Уникнення W8-підміни вище не повинне ламати справжні D3/D4 поверхні.
    let джерело = "(визначити хід (функція (x) (за-умовою ((атом? x) (сполучити x (як-є ()))))))";
    let форми = parse_mixed_exact_domain(джерело).expect("чинна українська проєкція");
    assert_eq!(форми.len(), 1);

    let ExprKind::List(визначення) = &форми[0].kind else { panic!("визначення"); };
    assert!(matches!(&визначення[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 4 && id.packed_bits() == 0b0011));
    assert!(!matches!(&визначення[1].kind, ExprKind::DomainIdentity(_)),
        "ім'я має лишитися даними");

    let ExprKind::List(лямбда) = &визначення[2].kind else { panic!("лямбда"); };
    assert!(matches!(&лямбда[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 4 && id.packed_bits() == 0b0010));

    let ExprKind::List(умова) = &лямбда[2].kind else { panic!("умова"); };
    assert!(matches!(&умова[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b110));

    let ExprKind::List(гілка) = &умова[1].kind else { panic!("гілка"); };
    assert_eq!(гілка.len(), 2, "COND завжди має точні двопольові гілки");
    let ExprKind::List(перевірка) = &гілка[0].kind else { panic!("предикат"); };
    assert!(matches!(&перевірка[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b010));

    let ExprKind::List(пара) = &гілка[1].kind else { panic!("CONS"); };
    assert!(matches!(&пара[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b111));

    let ExprKind::List(цитата) = &пара[2].kind else { panic!("QUOTE"); };
    assert!(matches!(&цитата[0].kind, ExprKind::DomainIdentity(id)
        if id.width() == 3 && id.packed_bits() == 0b001));
}
