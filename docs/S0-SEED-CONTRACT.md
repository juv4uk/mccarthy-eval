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
ст.29-30). `GET`/`DEFLIST`/`REMPROP`/`FLAG`/`REMFLAG`, property lists
(ст.39-41, 58-59). `ARRAY`, "4.4 The Array Feature" (ст.27-28). Див.
окремі розділи нижче.

Разом: **59 іменованих символів**, кожен з provenance-цитатою на
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

Reader dotted-pair синтаксис на вході -- закрито, див. окремий розділ
нижче.

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

## Reader dotted-pair синтаксис на вході -- закрито (2026-09-22)

Printer вже давно виробляв `(A . B)` на виході; reader не міг
розпізнати той самий синтаксис на вході -- `.` як окремий токен
читався буквально як символ, не як роздільник пари (задокументовано
під час роботи над `SUBLIS`, `tests/lisp15-library-functions/
PROVENANCE.md`). Виправлено: новий `peek_dot` helper в `read_list`
розпізнає відокремлений `.` і читає рівно один вираз як `cdr`.
Реальна знахідка: асиметрія була прихована візуальною колізією --
`(QUOTE (A . B))` на старому reader-і читався як звичайний
3-елементний список (`A`, `.`, `B`), друк якого через звичайний
список-принтер випадково давав ТОЙ САМИЙ рядок `(A . B)`, що й друк
справжньої 2-елементної пари. Реальна різниця видна лише структурно
(`CDR` справжньої пари дає `B`; `CDR` старого 3-списку давав `(. B)`).
Деталі, включно з чесним визнанням, які fixture не дискримінують
стару/нову версію -- `tests/reader-dotted-pairs/PROVENANCE.md`.

## `GET`/`DEFLIST`/`REMPROP`/`FLAG`/`REMFLAG` -- property lists закрито (2026-09-22)

Окрема тема з основного тіла маніфесту (ст.39-41, 58-59), не Appendix
A/B. `PUTPROP` (термін із пізніших діалектів Lisp) не існує в цьому
маніфесті 1962 року -- реальна функція з тим самим призначенням тут
називається `deflist`. Кожен символ отримує окремий, незалежний
property list (плаский, чергований список `(indicator value
indicator value ...)`, підтверджено власними діаграмами маніфесту),
збережений у новому `proplist_table`, index-паралельному до
`symtab`. Не моделюється "-1 sentinel head" чи internal
`PNAME`/`EXPR`/`SUBR` indicator-и, якими реальна система користується
для власного зберігання функцій -- 704/CTSS-специфічна деталь
представлення, якої цей kernel не поділяє (функції тут живуть у
`global_env`, не на property list).

**Самокорекція, а не лише знахідка**: перша версія цієї роботи
"виправила" `get`'s власну рекурсію на `cddr` (подвійний `cdr`),
аргументуючи, що одинарний `cdr` не може бути правильним для
чергованого списку. Під час реалізації `flag`/`remflag` виявилось, що
**та сама "корекція" була помилковою**: property list може містити
**flag** -- indicator без значення (ст.59). Одинарний `cdr` (точно як
відскановано, без "виправлення") насправді коректно узагальнює на
змішані списки flag-ів і пар; подвійний `cdr` ламається саме на них.
Виправлено назад на буквальний, відсканований `get[cdr[x];y]`. Усі
раніше написані fixture перевірено повторно без змін (жоден не
змішував flags із парами, тому баг довго лишався непоміченим).

`flag[l;ind]` кладе прапор (indicator без значення) на початок
property list кожного символу зі списку `l`, ідемпотентно ("no
property list ever receives a duplicated flag"); `remflag[l;ind]`
видаляє всі його входження, забираючи РІВНО один елемент за збіг (на
відміну від `remprop`, який забирає два -- indicator і значення, що
мало б бути невірним для прапора без значення).

Деталі, включно з повним журналом самокорекції -- `tests/appendix-b-
proplists/PROVENANCE.md`.

## `ARRAY` -- the Array Feature закрито (2026-09-23)

"4.4 The Array Feature" (ст.27-28), перевірено напряму по зображенню
сторінки. Реалізовано лише "LIST"-масиви -- маніфест сам каже, що
не-LIST масиви "reserved for future developments of the LISP system",
тобто LIST був єдиним видом, що колись реально існував. Storage --
звичайний Lisp-список NIL-комірок (O(n) доступ через `cdr`), не
справжній пакований вектор -- reconstruction-derived спрощення: цей
kernel не має окремої пам'яті для векторів, і маніфест не дає жодної
спостережуваної поведінки, яка б розрізняла ці два представлення.
`array[...]` архітектурно top-level-only, точно як `DEFINE` і з тієї
самої причини (мутація `global_env` напряму).

**Реальна знахідка й самовиправлений баг**: перша версія обчислення
лінійного індексу (`array_linear_index`) реально **зависала** на
будь-якому масиві з 2+ вимірами -- `getfix` двічі викликався без
попереднього перенесення значення `car` в `%rdi`, тому розпаковував
СТАРИЙ вміст регістру (вказівник на список), а не щойно прочитану
координату/розмір -- даючи сміттєвий "індекс" (адреса пам'яті, зсунута
на 2 біти) і astronomically повільний, хоч технічно скінченний, обхід
через `cdr`. Один ручний тест на 1D-масиві випадково "пройшов" (малий
сміттєвий вказівник) ДО виправлення -- не доказ коректності, лише
2D-тест реально виявив баг. Виправлено; перевірено на 1D/2D/3D.
Деталі -- `tests/array-feature/PROVENANCE.md`.

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
- повний прогін усіх одинадцяти корпусів (McCarthy core, ISA-baseline,
  LISP 1.5 hardware primitives, LISP 1.5 library functions, REPL
  diagnostics, Appendix B FUNCTION/FUNARG, Appendix B PROG/GO, reader
  dotted-pairs, Appendix B property lists, Array Feature) двічі --
  output-identity
  доказ.

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

## S0 реально виконує Core 1 -- empirically confirmed, 2026-09-23

`core1.lisp` більше не гіпотетичний. `my-lisp#1132` закрито; `lib/core1.lisp`,
`tests/fixtures/core1-s0-witness.lisp` і CI workflow `.github/workflows/
core1-s0.yml` злиті в `my-lisp` main (`ca219968`). Цей workflow **буквально
завантажує та збирає цей репозиторій** на закріпленому коміті `1ae9745`
(PR #47, `FUNCTION`/`FUNARG`) і виконує через нього справжній `core1.lisp`
-- Core1's власний `LAMBDA` представлено через `(C1-CLOSURE params body
lexical-env)`, транслюючись у наш `FUNARG`-механізм.

Перевірено НЕ лише читанням CI YAML, а прямим локальним відтворенням
(окрема scratch-копія `mccarthy-eval` на коміті `1ae9745`, зібрана
тим самим `gcc -no-pie -O0 -s`, `lib/core1.lisp` +
`core1-s0-witness.lisp` конкатеновані й подані на вхід):

```
HISTORICAL-FUNARG
T
(A . B)
C
(C1-ERROR UNBOUND VECTOR)
```

Точний збіг з очікуваним значенням у CI. Другий, глибший тест
(S0→S1→S2: `core1.lisp` виконує `wsm-my-lisp`'s `compiler.lisp` на
закріпленому коміті `a6bd9747`) також відтворено локально й дав
точний очікуваний результат `(var FOO)`.

Жодних змін до асемблера не знадобилось -- Core1 навмисно
використовує лише історичну S0-поверхню (`lib/core1.lisp`'s власний
коментар: "no new assembler semantics are requested", "first-class
closure capture is represented as ordinary Lisp data"). Межа
збережена: `mccarthy-eval` лишається historical reconstruction/
witness, не стає my-lisp semantic authority -- `my-lisp#1132`
сформулював і реалізував Core1's мовну семантику сам, цей репозиторій
лише виконує її на реальному залізі.

`ecosystem#15`'s S0→S5 драбина тепер має робочий, empirically
підтверджений перший крок.
