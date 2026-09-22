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
MEMBER APPEND MAPLIST PAIR SASSOC SEARCH REVERSE LENGTH COPY SUBLIS`.

**LISP 1.5 Appendix B (1962) -- інтерпретатор**: `FUNCTION`/`FUNARG`,
реальний closure-механізм 1962 року (ст.70-71 маніфесту, перевірено
напряму по зображенню сторінки). `PROG`/`GO`/`RETURN`/`SETQ`/`SET`,
Algol-подібний program feature (ст.71, і головне -- канонічні приклади
`LENGTH`/`REV` з "V. THE PROGRAM FEATURE" в основному тілі маніфесту,
ст.29-30). Див. окремі розділи нижче.

Разом: **52 іменованих символи**, кожен з provenance-цитатою на
конкретну сторінку першоджерела. Жодна названа тут форма не
претендує на Core 4 семантику.

## Closures/FUNARG gap -- частково закрито (2026-09-22), межа звужена й задокументована

**Раніше** цей документ звітував: "у цьому seed немає first-class
функцій / lexical closures" узагалі, з прикладом `((LABEL MAKE-ADDER
(LAMBDA (N) (LAMBDA (X) (PLUS X N)))) 5) -> NIL`. Це лишається
ПРАВДОЮ буквально для цього прикладу (голий, не загорнутий `LAMBDA` як
значення), але виявилось, що **реальна LISP 1.5 (1962) сама має
точно той самий gap і точно той самий, задокументований спосіб його
обійти** — `FUNCTION`/`FUNARG`, Appendix B ст.70-71. Це не було
додано "заради зручності" (заборонено нижче, пункт 3) — це
реалізація вже наявної в самому джерелі 1962 року конструкції,
`tests/appendix-b-funarg/PROVENANCE.md` містить повне обґрунтування й
adversarial-перевірку.

**Що працює зараз** (empirically confirmed, `tests/appendix-b-funarg/`,
6 fixture): функція, що явно загортає свою власну λ через `(FUNCTION
(LAMBDA ...))` перед поверненням, стає справжнім, коректно
захопленим closure-значенням -- включно з вкладеним каррінгом (дві
незалежні рівні захоплення одночасно) і передачею closure як
параметра в іншу функцію:

```
(DEFINE MAKE-ADDER (LAMBDA (N) (FUNCTION (LAMBDA (X) (PLUS X N)))))
(DEFINE ADD5 (MAKE-ADDER 5))
(ADD5 3) -> 8
```

**Що досі НЕ працює, свідомо, історично точно** -- межа звужена, не
стерта:

```
((LABEL MAKE-ADDER (LAMBDA (N) (LAMBDA (X) (PLUS X N)))) 5)
-> CONDITION kind=UNBOUND name=LAMBDA
-> NIL
```

Голий `(LAMBDA ...)`, повернутий як значення БЕЗ явного `FUNCTION`,
досі провалюється -- і це не залишковий баг: реальна LISP 1.5 1962
року теж не дозволяє це (`eval` перевіряє лише `eq[car[form],
FUNCTION]`, ніколи `LAMBDA`, коли `form` не є буквальним викликом).
Adversarially підтверджено (`tests/appendix-b-funarg/PROVENANCE.md`):
цей негативний приклад дає однаковий результат і до, і після
додавання `FUNCTION`/`FUNARG`.

Все ще НЕ реалізовано й НЕ заплановано: reader не парсить dotted-pair
синтаксис на вході (`tests/lisp15-library-functions/PROVENANCE.md`).

**Якщо Core 1 потребує closures, що НЕ використовують явний
`FUNCTION`** (наприклад, автоматичне захоплення вільних змінних без
явної обгортки, як у сучасних Lisp-ах) -- цей seed їх НЕ дасть, і це
було б справді новою семантикою, не історичною реконструкцією; така
потреба або узгоджується окремо, або S1 (`core1.lisp`) пишеться з
явним `FUNCTION` там, де потрібне справжнє замикання (як і в
реальному коді 1962 року).

## `PROG`/`GO`/`RETURN`/`SETQ`/`SET` -- закрито (2026-09-22)

Algol-подібний program feature (ст.71 Appendix B; канонічні приклади
`LENGTH`/`REV`, ст.29-30 основного тіла) тепер реалізовано і
adversarially перевірено, `tests/appendix-b-prog/` (8 fixture).
Реалізовано без нової семантики понад те, що описує сам маніфест:
`PROG` зв'язує program-змінні (початково `NIL`), виконує оператори
послідовно, `GO` перестрибує на мітку (пересканує тіло з початку,
reconstruction-derived еквівалент внутрішньої go-list структури
реальної системи -- той самий підхід, що вже застосований до
`PAIR`/`REVERSE`), `RETURN` завершує `PROG` зі значенням, `SETQ`/`SET`
мутують існуюче зв'язування на a-list напряму (не створюють нове).

**Реальна знахідка під час перевірки**: маніфест сам суперечить собі
в деталі -- Appendix B (ст.71) документує лише `GO` як дозволену
значення-частину top-level `COND`, але власний канонічний приклад
`LENGTH` (ст.29-30) використовує `(COND ((NULL U) (RETURN V)))` --
`RETURN`, не `GO`. Перша реалізація, буквально за текстом Appendix B,
дала реальний зависаючий цикл на цьому самому прикладі. Виправлено:
`COND`-в-`PROG` розпізнає обидва, `GO` і `RETURN`, як
значення-частину -- емпірично необхідне, підтверджене буквальним
прикладом з того самого джерела, не вигадка. Деталі, включно з
чесним визнанням, які fixture НЕ дискримінують стару/нову версію
kernel (ті, чия правильна відповідь `NIL` збігається з випадковим
`NIL` старої версії з іншої причини) -- `tests/appendix-b-prog/
PROVENANCE.md`.

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
- повний прогін усіх восьми корпусів (McCarthy core, ISA-baseline,
  LISP 1.5 hardware primitives, LISP 1.5 library functions, REPL
  diagnostics, Appendix B FUNCTION/FUNARG, Appendix B PROG/GO) двічі
  -- output-identity доказ.

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
