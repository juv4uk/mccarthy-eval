# Provenance: adopted historical-facility extension corpus (issue #27)

Окремий від `tests/historical-core/` (#9) корпус -- перевіряє
adopted later historical facilities (`PLUS`/`TIMES`/`DIFFERENCE`/
`ZEROP`), не historical-core Маккарті 1960. Жоден historical-core
fixture тут не змінено й не залежить від цього корпусу.

## Рішення issue #27: реалізувати чи narrowed?

- **`PLUS`/`TIMES` -- реалізовано source-faithful n-ary**, точно за
  LISP 1.5 Programmer's Manual (1962), §4.2, ст.31-32: "is a function
  of any number of arguments". Зворотньо сумісно -- `n=2` дає той
  самий результат, що й попередня фіксована 2-арна версія (жоден
  наявний `.lisp`-файл цього репо не зламано, перевірено:
  `demo.lisp`, `factorial.lisp`, `listutils.lisp` дають ідентичний
  вивід до і після зміни).
- **`DIFFERENCE`, `ZEROP` -- без змін**, уже source-faithful
  (`DIFFERENCE` бінарна і в джерелі; `ZEROP` навмисно покриває лише
  fixed-point гілку джерельного правила, бо kernel має лише integers
  -- обґрунтування зафіксоване раніше в #22/`docs/HISTORICAL-FUNCTION-LEDGER.md`).
- **`DEFINE` -- свідомо залишено narrowed**, НЕ реалізовано
  batch-форму джерела. Причина: історична форма — `DEFINE
  ((name1 . expr1) (name2 . expr2) ...)` -- це інша calling
  convention (один вкладений список пар, не `(DEFINE name expr)`),
  несумісна з поточною. Заміна зламала б **усі** наявні `.lisp`-файли
  цього репо (`demo.lisp`, `listutils.lisp`, `factorial.lisp`,
  `startup.lisp`), які використовують встановлену пласку форму, заради
  можливості, якою жоден historical-core чи extension fixture не
  користується. Це відповідає власному дозволу issue #27 залишити
  extension narrowed, коли source-faithful реалізація ламає наявну
  функціональність без потреби.

## Fixtures

| Fixture | Перевіряє | Провенанс |
|---|---|---|
| `01-plus-nary-five-args` | `PLUS` на 5 аргументах | LISP 1.5, §4.2, ст.31 |
| `02-times-nary-three-args` | `TIMES` на 3 аргументах | LISP 1.5, §4.2, ст.32 |
| `03-plus-unary-identity` / `04-times-unary-identity` | 1-аргументний виклик (identity fold) | reconstruction-derived witness, пряма інстанціація "any number of arguments" |
| `05-plus-zero-arity` / `06-times-zero-arity` | 0-аргументний виклик -- `PLUS`→0, `TIMES`→1 (identity елементи) | reconstruction-derived witness, пряма інстанціація "any number of arguments" (включно з нулем) |
| `07-plus-nested-times` | композиція n-арних форм | reconstruction-derived witness |
| `08-difference-binary-fidelity` | regression -- `DIFFERENCE` не зачепило зміну PLUS/TIMES | LISP 1.5, §4.2, ст.31 |
| `09-zerop-fixed-point-true` / `10-zerop-fixed-point-false` | regression -- `ZEROP` не зачепило зміну | LISP 1.5, §4.2, ст.32 |

## Verification

`.expected` згенеровано напряму з реального виконання зібраного
x86-64 бінарника. Перевірено додатково: повний `#9`
historical-core corpus (21/21) і ISA-baseline witness (`tests/
isa-baseline/`, статичний + QEMU/Conroe) -- обидва без змін після
модифікації `.try_plus`/`.try_times`, нова n-арна реалізація так само
не використовує жодного gated-розширення CPU (лише `push`/`pop`/
`imul`/`add`/`cmp`/`jmp`/`call` -- baseline x86-64 ціле-числова
арифметика).
