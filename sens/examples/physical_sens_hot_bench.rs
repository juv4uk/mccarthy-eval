//! Гарячий бенчмарк чинної двійкової мови. Вимірює фазу, а не CLI startup.
//! Rust не затверджує жодного мовного закону: лише запускає вже чинні
//! T5/D2/нижчі механізми і порівнює їхні спостережувані результати.

use sens::{
    decode_ternary_words, encode_ternary_words, eval_lowered_expressions, eval_parsed_expressions, lower_program,
    open_ternary_program, render_ternary_words_spaced, parse_canonical_binary, parse_canonical_packed_words,
    parse_binary_source_words, parse_canonical_word_sequence, pack_binary_source_tokens, pack_binary_source_words, Session,
};
use std::{env, fs, hint::black_box, time::Instant};

const PHASES: &[&str] = &["t5_render_legacy", "t5_render_one_pass", "t5_open_d2", "t5_visible_parse_d2", "t5_direct_d2", "t5_words_d2", "d2_parse", "packed_width_d2", "eval_from_ast", "eval_lowered", "t5_encode_two_pass", "t5_encode_streaming"];

/// Попередній двопрохідний алгоритм тільки для порівняння механіки.
/// Жодна T5-цифра не є мовним резидентом; виконуваний код не викликає цей
/// контрольний варіант. Порівняння байтів відбувається ДО таймінгу.
fn two_pass_t5_allocation_control(words: &[sens::BinarySourceWord]) -> Vec<u8> {
    let mut trits = Vec::new();
    for (position, word) in words.iter().copied().enumerate() {
        if position != 0 {
            trits.push(2);
        }
        let payload = word.packed_bits();
        for shift in (0..word.width()).rev() {
            trits.push(((payload >> shift) & 1) as u8);
        }
    }
    while trits.len() % 5 != 0 {
        trits.push(2);
    }
    trits.chunks_exact(5).map(|five| {
        five.iter().fold(0u16, |acc, &digit| acc * 3 + u16::from(digit)) as u8
    }).collect()
}

/// Старий людський перегляд: для кожного слова окремий String та
/// додаткова тимчасова колекція. Використовується тільки в бенчмарку.
fn legacy_visible_word_renderer(words: &[sens::BinarySourceWord]) -> String {
    words.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")
}

fn median_ns(mut xs: Vec<u128>) -> (u128, u128, u128) {
    xs.sort_unstable();
    let mid = xs[xs.len() / 2];
    // Найближчий ранг, не інтерпольований — p95 на фактичних замірах.
    let p95_index = (95 * xs.len()).div_ceil(100).saturating_sub(1);
    (mid, xs[p95_index], xs[0])
}

fn measure(mut f: impl FnMut(), count: usize, samples: usize) -> Vec<u128> {
    for _ in 0..count.min(100) {
        f();
    }
    let mut observations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        for _ in 0..count {
            f();
        }
        observations.push(started.elapsed().as_nanos() / count as u128);
    }
    observations
}

fn main() {
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 5, "usage: physical_sens_hot_bench FILE.sens FORMS WORK_BUDGET SAMPLES");
    let forms = args[2].parse::<usize>().expect("forms");
    let work_budget = args[3].parse::<usize>().expect("work budget");
    let samples = args[4].parse::<usize>().expect("samples");
    assert!((1..=2048).contains(&forms));
    assert!((4..=100_000_000).contains(&work_budget));
    assert!((3..=101).contains(&samples) && samples % 2 == 1);
    let count = (work_budget / forms).max(4);

    let physical = fs::read(&args[1]).expect("фізичний T5");
    let visible = open_ternary_program(&physical).expect("чинний фізичний T5/D2");
    let parsed = parse_canonical_binary(&visible).expect("канонічний D2");
    assert_eq!(parsed.len(), forms, "одна D3 QUOTE форма на один елемент");
    // Prepare one genuinely dense exact-width payload (not the T5 container).
    // Word boundaries come from the already-admitted source; deriving them
    // from bit patterns would silently invent a new semantic framing law.
    let tokens = parse_binary_source_words(&visible).expect("ratified word lexer");
    let widths: Vec<usize> = tokens.iter().map(|word| word.word.width()).collect();
    let dense = pack_binary_source_tokens(&tokens);
    let packed_parsed = parse_canonical_packed_words(&dense, &widths)
        .expect("direct dense D2 reader");
    assert_eq!(packed_parsed.len(), forms);
    // Direct physical T5 bytes to exact D2 words, without making a string.
    // The T5 separator trits supply the word boundaries; no independent
    // width discovery, new parser or human semantic name table is involved.
    let t5_words = decode_ternary_words(&physical).expect("raw physical T5 words");
    assert_eq!(two_pass_t5_allocation_control(&t5_words), physical,
        "two-pass measurement control differs from source physical T5");
    assert_eq!(encode_ternary_words(&t5_words).expect("current T5 encoder"), physical,
        "streaming T5 encoder differs from source physical T5");

    // Біт за бітом, без інтерпретації: нова одноалокаційна проєкція
    // має бути рівною початковому формату для кожного реального T5.
    assert_eq!(
        legacy_visible_word_renderer(&t5_words),
        render_ternary_words_spaced(&t5_words),
        "оптимізація не може змінити exact-width вивід"
    );
    let t5_widths = t5_words.iter().map(|word| word.width()).collect::<Vec<_>>();
    let t5_dense = pack_binary_source_words(&t5_words);
    let t5_parsed = parse_canonical_packed_words(&t5_dense, &t5_widths)
        .expect("direct physical T5 into D2");
    assert_eq!(t5_parsed.len(), forms, "direct T5 form count drift");
    let word_parsed = parse_canonical_word_sequence(&t5_words)
        .expect("direct typed-word D2 reader");
    assert_eq!(word_parsed.len(), forms, "typed word form count drift");
    // Fair end-to-end T5->AST comparator: unlike t5_open_d2, this
    // completes the canonical visible-word parse and returns an actual AST.
    let text_round_trip = open_ternary_program(&physical).expect("text T5 view");
    let visible_parsed = parse_canonical_binary(&text_round_trip)
        .expect("T5 -> text -> D2 AST");
    assert_eq!(visible_parsed.len(), forms, "visible T5 form count drift");
    let lowered = lower_program(&parsed);
    let mut session = Session::default();

    // Наявні канонічні шляхи мають давати однаковий observable, до вимірювання.
    let ast_observable = eval_parsed_expressions(&parsed, &mut session)
        .expect("канонічне виконання AST");
    let lowered_observable = eval_lowered_expressions(&lowered, &mut session)
        .expect("виконання вже зниженого AST");
    assert_eq!(ast_observable, lowered_observable, "AST/lowered observable mismatch");
    let packed_observable = eval_parsed_expressions(&packed_parsed, &mut session)
        .expect("direct packed-word D2 execution");
    assert_eq!(ast_observable, packed_observable, "visible/packed D2 observable mismatch");
    let t5_observable = eval_parsed_expressions(&t5_parsed, &mut session)
        .expect("direct T5 D2 execution");
    assert_eq!(ast_observable, t5_observable, "physical T5 direct D2 observable mismatch");
    let word_observable = eval_parsed_expressions(&word_parsed, &mut session)
        .expect("typed T5 source words execution");
    assert_eq!(ast_observable, word_observable, "typed/direct/visible D2 observable mismatch");
    let visible_observable = eval_parsed_expressions(&visible_parsed, &mut session)
        .expect("T5 to visible D2 execution");
    assert_eq!(ast_observable, visible_observable,
        "T5 visible and binary direct execution must agree");
    let stable = ast_observable.value.to_string();
    black_box(&stable);

    for &phase in PHASES {
        let observations = match phase {
            "t5_render_legacy" => measure(
                || { black_box(legacy_visible_word_renderer(black_box(&t5_words))); },
                count, samples,
            ),
            "t5_render_one_pass" => measure(
                || { black_box(render_ternary_words_spaced(black_box(&t5_words))); },
                count, samples,
            ),
            "t5_open_d2" => measure(
                || { black_box(open_ternary_program(black_box(&physical)).expect("T5/D2")); },
                count, samples,
            ),
            "t5_visible_parse_d2" => measure(
                || {
                    let text = open_ternary_program(black_box(&physical))
                        .expect("T5 visible view");
                    black_box(parse_canonical_binary(&text).expect("visible D2 AST"));
                },
                count, samples,
            ),
            "t5_direct_d2" => measure(
                || {
                    let words = decode_ternary_words(black_box(&physical)).expect("T5 bytes");
                    let widths = words.iter().map(|word| word.width()).collect::<Vec<_>>();
                    let bits = pack_binary_source_words(&words);
                    black_box(parse_canonical_packed_words(&bits, &widths).expect("direct T5/D2"));
                },
                count, samples,
            ),
            "t5_words_d2" => measure(
                || {
                    let words = decode_ternary_words(black_box(&physical)).expect("T5 bytes");
                    black_box(parse_canonical_word_sequence(&words).expect("typed T5/D2"));
                },
                count, samples,
            ),
            "d2_parse" => measure(
                || { black_box(parse_canonical_binary(black_box(&visible)).expect("D2")); },
                count, samples,
            ),
            "packed_width_d2" => measure(
                || {
                    black_box(
                        parse_canonical_packed_words(
                            black_box(&dense),
                            black_box(&widths),
                        )
                        .expect("direct dense D2"),
                    );
                },
                count, samples,
            ),
            "eval_from_ast" => measure(
                || {
                    let result = eval_parsed_expressions(black_box(&parsed), &mut session)
                        .expect("AST execution");
                    black_box(result);
                }, count, samples,
            ),
            "t5_encode_two_pass" => measure(
                || {
                    black_box(two_pass_t5_allocation_control(black_box(&t5_words)));
                }, count, samples,
            ),
            "t5_encode_streaming" => measure(
                || {
                    black_box(encode_ternary_words(black_box(&t5_words))
                        .expect("streaming T5 encoder"));
                }, count, samples,
            ),
            "eval_lowered" => measure(
                || {
                    let result = eval_lowered_expressions(black_box(&lowered), &mut session)
                        .expect("lowered execution");
                    black_box(result);
                }, count, samples,
            ),
            _ => unreachable!(),
        };
        let (median, p95, minimum) = median_ns(observations.clone());
        for (rep, elapsed_ns) in observations.iter().enumerate() {
            println!(
                "HOT_SAMPLE\tforms={forms}\tphase={phase}\trep={}\tns_op={elapsed_ns}",
                rep + 1,
            );
        }
        println!(
            "HOT_BENCH\tforms={forms}\tphase={phase}\titerations={count}\tsamples={samples}\tmedian_ns_op={median}\tp95_ns_op={p95}\tmin_ns_op={minimum}\tphysical_bytes={}\tobservable={stable:?}",
            physical.len(),
        );
    }
}
