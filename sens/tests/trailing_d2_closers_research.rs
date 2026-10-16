//! Незалежний Rust-оракул T5 на ратифікованому читачі; без зміни production.
use sens::parse_canonical_binary;

fn remove_terminal_close<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let mut end = words.len();
    while end > 0 && words[end - 1] == "01" {
        end -= 1;
    }
    words[..end].to_vec()
}

fn complete_from_depth(words: &[&str]) -> Vec<String> {
    let mut depth = 0i32;
    let mut result: Vec<String> = Vec::from_iter(words.iter().map(|s| s.to_string()));
    for word in words {
        match *word {
            "10" => depth += 1,
            "01" => depth -= 1,
            _ => {}
        }
        assert!(depth >= 0);
    }
    result.extend(std::iter::repeat_n("01".to_string(), depth as usize));
    result
}

#[test]
fn ratified_d1_through_d9_roundtrip_through_real_reader() {
    let mut examined = 0usize;
    for width in [1usize, 3, 4, 5, 6, 7, 8, 9] {
        for value in 0usize..(1 << width) {
            if width == 7 && [0b0100001, 0b0101010].contains(&value) {
                continue; // дві зарезервовані координати D7
            }
            examined += 1;
            let payload = format!("{value:0width$b}");
            for words in [
                vec!["10", payload.as_str(), "01"],
                vec!["10", "10", payload.as_str(), "01", "01"],
                vec![payload.as_str(), "10", "001", payload.as_str(), "01"],
                vec!["10", "0", "11", payload.as_str(), "01"],
            ] {
                let raw = words.join(" ");
                assert!(parse_canonical_binary(&raw).is_ok(), "valid W{width}: {raw}");
                let compact = remove_terminal_close(&words);
                let back = complete_from_depth(&compact);
                assert_eq!(
                    back.iter().map(String::as_str).collect::<Vec<_>>(),
                    words,
                    "W{width}: {raw}"
                );
                let reproduced = back.join(" ");
                assert!(parse_canonical_binary(&reproduced).is_ok(), "reader: {reproduced}");
                if compact.len() != words.len() {
                    assert!(parse_canonical_binary(&compact.join(" ")).is_err());
                }
            }
        }
    }
    assert_eq!(examined, 1016); // точна місткість ратифікованих payload-доменів
}

#[test]
fn d2_separator_and_dotted_pair_require_explicit_canonical_reconstruction() {
    for words in [
        vec!["10", "01"],
        vec!["10", "00", "01"],
        vec!["10", "0", "11", "1", "01"],
        vec!["10", "0", "11", "10", "1", "01", "01"],
        vec!["10", "0", "01", "10", "1", "01"],
        vec!["10", "001", "10", "000", "01", "01"],
    ] {
        assert!(parse_canonical_binary(&words.join(" ")).is_ok());
        let compact = remove_terminal_close(&words);
        let restored = complete_from_depth(&compact);
        assert_eq!(restored.iter().map(String::as_str).collect::<Vec<_>>(), words);
    }
    for invalid in [
        "10 0 11 01",
        "10 11 1 01",
        "10 0 11 0 1 01",
        "10 01 01",
        "10 0000000000 01", // unratified W10/D10
    ] {
        assert!(parse_canonical_binary(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn bare_eof_after_an_unclosed_list_is_not_admitted_by_canonical_reader() {
    // Це демонструє, чому скорочення потребує НОВОГО фізичного профілю.
    for truncated in ["10", "10 10", "10 111", "10 0 11 1"] {
        assert!(parse_canonical_binary(truncated).is_err());
        let words: Vec<&str> = truncated.split_whitespace().collect();
        assert!(parse_canonical_binary(&complete_from_depth(&words).join(" ")).is_ok());
    }
    // Пошкодження довшого короткого запису може дати інший правильний.
    assert_eq!(complete_from_depth(&["10"]), vec!["10", "01"]);
    assert_eq!(
        complete_from_depth(&["10", "10"]),
        vec!["10", "10", "01", "01"]
    );
}


/// Лише справжні структурні слова D2, не довільні 10 усередині D3–D9.
fn all_leading_opens(words: &[&str]) -> usize {
    words.iter().take_while(|&&w| w == "10").count()
}

fn trim_all_ends<'a>(words: &'a [&'a str]) -> (usize, Vec<&'a str>) {
    let k = all_leading_opens(words);
    let mut end = words.len();
    while end > k && words[end - 1] == "01" {
        end -= 1;
    }
    (k, words[k..end].to_vec())
}

/// Відновити повний D2 з явно збереженої кількості початкових OPEN.
fn restore_all_ends(short: &[&str], k: usize) -> Vec<String> {
    let mut restored = vec!["10".to_string(); k];
    restored.extend(short.iter().map(|s| s.to_string()));
    let mut depth = 0i64;
    for w in &restored {
        match w.as_str() {
            "10" => depth += 1,
            "01" => depth -= 1,
            _ => {}
        }
        assert!(depth >= 0, "неправильний структурний D2");
    }
    restored.extend(std::iter::repeat_n("01".to_string(), depth as usize));
    restored
}

#[test]
fn all_initial_opens_and_all_terminal_closes_are_reversible_with_explicit_count() {
    // Той самий справжній ратифікований читач, що й в інших D2-тестах.
    for depth in [1usize, 2, 3, 8, 16, 32, 64] {
        for payload in ["0", "000", "000000000"] {
            let mut program = vec!["10"; depth];
            program.push(payload);
            program.extend(std::iter::repeat_n("01", depth));
            assert!(parse_canonical_binary(&program.join(" ")).is_ok());
            let (count, short) = trim_all_ends(&program);
            assert_eq!(count, depth);
            assert_eq!(short, [payload]);
            let recovered = restore_all_ends(&short, count);
            assert_eq!(recovered.iter().map(String::as_str).collect::<Vec<_>>(), program);
            assert!(parse_canonical_binary(&recovered.join(" ")).is_ok());
        }
    }
    for words in [
        vec!["10", "01", "10", "01"], // два корені, префікс видаляємо із лічильником
        vec!["10", "10", "000", "01", "01", "10", "1", "01"],
        vec!["10", "0", "11", "1", "01"], // крапкова пара
        vec!["000", "10", "0", "01"], // атом спочатку: count=0
        vec!["10", "01", "00"], // кінцевий D2 пропуск
    ] {
        assert!(parse_canonical_binary(&words.join(" ")).is_ok(), "{words:?}");
        let (k, short) = trim_all_ends(&words);
        let recovered = restore_all_ends(&short, k);
        assert_eq!(recovered.iter().map(String::as_str).collect::<Vec<_>>(), words);
        assert!(parse_canonical_binary(&recovered.join(" ")).is_ok());
    }
}

#[test]
fn without_initial_open_count_two_distinct_programs_collapse_to_the_same_body() {
    let left = ["10", "0", "01"];
    let right = ["10", "10", "0", "01", "01"];
    assert!(parse_canonical_binary(&left.join(" ")).is_ok());
    assert!(parse_canonical_binary(&right.join(" ")).is_ok());
    let (k_left, stripped_left) = trim_all_ends(&left);
    let (k_right, stripped_right) = trim_all_ends(&right);
    assert_eq!(stripped_left, stripped_right);
    assert_ne!(k_left, k_right);
    assert_eq!(restore_all_ends(&stripped_left, k_left), left);
    assert_eq!(restore_all_ends(&stripped_right, k_right), right);
}
