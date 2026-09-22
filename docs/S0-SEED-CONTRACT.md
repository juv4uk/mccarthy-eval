# S0 seed contract для Core 1 (issue #42, parent ecosystem#15)

Цей документ готує `mccarthy-eval` до ролі **S0 фізичного seed** у
bootstrap-драбині:

```
S0 mccarthy-eval (фізичний seed)
  -> S1 core1.lisp виконує Lisp-owned eval/apply/bootstrap
  -> S2 compiler source, написаний Core 1 Lisp
  -> S3 compiler artifact, вироблений seed-шляхом
  -> S4 S3 перекомпільовує той самий compiler source
  -> S5 незалежне порівняння S3/S4
```

`mccarthy-eval` лишається historical reconstruction/witness. Він
**не** стає my-lisp semantic authority і **не** приймає Core 4
семантику мовчки — обидва обмеження явно записані в issue #42 і
підтверджені `AGENTS.md` цього репо ще до цієї програми.

## Що seed реально пропонує зараз (source-confirmed, не здогад)

Повний перелік із точними джерелами -- `docs/HISTORICAL-FUNCTION-LEDGER.md`,
`tests/lisp15-library-primitives/PROVENANCE.md`,
`tests/lisp15-library-functions/PROVENANCE.md`. Коротко:

**McCarthy 1960 core** (CACM 1960, ст.16-18): `QUOTE ATOM EQ COND CAR
CDR CONS LABEL LAMBDA` -- пряме застосування функції як голови
виклику, іменована рекурсія через `LABEL`.

**LISP 1.5 Appendix A (1962) -- asm-примітиви**: `NULL EQUAL LIST AND
OR NOT RPLACA RPLACD` + повна арифметика (`MINUS ADD1 SUB1 MAX MIN
RECIP QUOTIENT REMAINDER DIVIDE EXPT LESSP GREATERP ONEP MINUSP
NUMBERP FIXP FLOATP LOGOR LOGAND LOGXOR LEFTSHIFT`).

**LISP 1.5 Appendix A -- library-функції через LABEL/LAMBDA**: `SUBST
MEMBER APPEND MAPLIST`.

Разом: **45 іменованих символів**, кожен з provenance-цитатою на
конкретну сторінку першоджерела. Жодна названа тут форма не
претендує на Core 4 семантику.

## Явно повідомлений, не замовчаний gap: немає справжніх closures

**У цьому seed немає first-class функцій / lexical closures.**
Перевірено напряму:

```
((LABEL MAKE-ADDER (LAMBDA (N) (LAMBDA (X) (PLUS X N)))) 5)
-> CONDITION kind=UNBOUND name=LAMBDA
-> NIL
```

`LAMBDA` розпізнається лише як голова БЕЗПОСЕРЕДНЬОГО виклику
(`eq[caar[e];LAMBDA]` у формулі `eval`), ніколи як значення, що
повертається чи створюється динамічно. Єдиний "виклик через ім'я
першого класу" в цьому seed -- вузький окремий випадок: `DEFINE`
зберігає сирий `(LABEL name (LAMBDA ...))` у `global_env`, і коли це
ім'я пізніше передається як параметр (як `PAIRUP` у `listutils.lisp`
чи `WRAPFIRST` у `tests/lisp15-library-functions/`), `plain_call`
підставляє сирий вираз назад у `eval`. Це працює **тільки** для
top-level `DEFINE`'d імен -- не для довільно сконструйованих
closures, що захоплюють змінне лексичне середовище (класична
"FUNARG problem" з історії Lisp 1960-х, та сама проблема, над якою
окремо працює `cml#180`, "general first-class application").

**Якщо Core 1 потребує справжніх closures -- цей seed їх НЕ дасть.**
Це report, не guess: `#42`'s acceptance criteria прямо вимагає "no
new Lisp semantics are introduced in assembly solely to make
bootstrap convenient" -- тому цей seed свідомо не буде імітувати
closures додаванням нової асемблерної семантики. Якщо Core 1
справді потребує first-class functions на S0-рівні, це або
узгоджується як явне розширення з окремим historical чи
reconstruction-derived provenance, або S1 (`core1.lisp`) має бути
написаний так, щоб не покладатись на них до S2/S3, де реальне
рішення (CML) уже працює над цим окремо.

## Реальна S0 evidence -- вже готова, не обіцянка

`tests/closeout/run.sh` уже виробляє точно те, що `#42` просить
("export reproducible S0 evidence: source SHA, build toolchain,
target CPU, binary hash, bootstrap transcript"):

- toolchain versions (gcc/binutils/qemu);
- `mccarthy-kernel.s` git HEAD + blob SHA-256;
- machine profile (CPU model, з попередженням, якщо не заявлений
  i5-6400);
- binary SHA-256, перевірена на детермінованість (два незалежні
  білди дають identical hash);
- повний прогін усіх шести корпусів (McCarthy core, ISA-baseline,
  LISP 1.5 hardware primitives, LISP 1.5 library functions, REPL
  diagnostics) двічі -- output-identity доказ.

Коли `core1.lisp` з'явиться в `my-lisp`, транскрипт його виконання
на цьому seed додається до того самого звіту як ще одна ворота --
структура вже готова, лише новий `run_gate "core1-bootstrap" ...`
рядок.

## Обмеження, які seed бере на себе явно (не мовчки)

1. Не стає my-lisp semantic authority.
2. Не приймає Core 4 (post-revolution my-lisp) семантику.
3. Не вигадує нову мовну семантику заради зручності bootstrap-у.
4. Кожна форма, яку `core1.lisp` реально використовує, має бути
   звірена проти цього документа й `docs/HISTORICAL-FUNCTION-LEDGER.md`
   -- форма без відповідника тут або має окреме provenance-обґрунтування,
   або явно звітується як unsupported (не guess, не silent emulation).

## Заблоковано, не моя робота

`core1.lisp` ще не існує ніде в `my-lisp` (перевірено напряму,
2026-09-22). Визначення точного мовного підмножини Core 1 -- робота
`my-lisp#1132` ("my-lisp remains the single semantic authority", за
`ecosystem#15`). Цей seed готовий виконати `core1.lisp`, щойно він
з'явиться -- жодних дій із мого боку до того часу, окрім цього
readiness-контракту.
