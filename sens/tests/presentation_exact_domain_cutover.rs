//! Перевірки чинного доменного подання та доказового переносу D3.
//! Зовнішня інтеграція компілює бібліотеку без cfg(test)-залежностей
//! інших зайнятих агентами модулів. Не є свідком виклику функції D8.

use sens::{
    Bit3, Bit9, Bija3, CoreD9, DomainIdentity,
    PresentationLanguage, Value, render_value_for_presentation,
};

#[test]
fn istorichnyi_quote_maie_lyshe_dovedenu_d3_proiektsiiu() {
    let old = Value::Sid(sens::sens!(00000001));
    let exact = Value::DomainIdentity(DomainIdentity::D3(
        Bija3::from_word(Bit3::new(0b001).unwrap()),
    ));
    assert_eq!(
        render_value_for_presentation(&old, PresentationLanguage::Ukrainian),
        render_value_for_presentation(&exact, PresentationLanguage::Ukrainian),
    );
    // Доведена людська проєкція — не зміна історичного wire.
    assert_eq!(old.to_string(), "00000001");
    assert_eq!(exact.to_string(), "001");
}

#[test]
fn d9_bez_runtime_surface_ne_vtrachaie_shyrynu_u_ukrainskomu_podanni() {
    let d9 = Value::DomainIdentity(DomainIdentity::D9(
        CoreD9::from_word(Bit9::new(0b100000001).unwrap()),
    ));
    assert_eq!(
        render_value_for_presentation(&d9, PresentationLanguage::Ukrainian),
        "#<домен D9:100000001>",
    );
    assert_eq!(
        render_value_for_presentation(&d9, PresentationLanguage::Canonical),
        "100000001",
    );
}
