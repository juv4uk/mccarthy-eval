# Provenance: LISP 1.5 Appendix A "hardware" primitives (issue #7, Фаза 2)

Джерело: **LISP 1.5 Programmer's Manual (1962), Appendix A, "Functions
and Constants in the LISP System... as of August 1962"** (ст.56-68) --
офіційний, повний каталог функцій від самих авторів, не реконструкція
за здогадом. Локальна копія: `external-sources/lisp-manuals/
LISP_1.5_Manual_1962.pdf`.

**Що саме тут, а що ні**: цей корпус містить лише функції, що
реалізовані як **реальні x86-64 машинні примітиви** -- нові гілки в
`eval`'s dispatch chain, справжні `idiv`/`imul`/`and`/`or`/`xor`/
`shl`/`sar`/`cmov` інструкції. Library-функції з Appendix A, чиї
M-вирази з тексту можна дослівно перекласти через `LABEL`/`LAMBDA`
без нових asm-примітивів (`subst`, `member`, `append`, `maplist`,
`sassoc`, `sublis`, `pair`, `reverse`, `length`, `copy`) -- окрема,
ще не виконана частина issue #7 Фази 2.

Кожен `.expected` згенеровано напряму з реального виконання зібраного
x86-64 бінарника (`run.sh`), не вручну.

## Elementary/logical primitives (ст.57-58)

| Fixture | Функція | Провенанс | Примітка |
|---|---|---|---|
| `01`/`02` | `NULL[x]` | Appendix A, ст.57 | предикат "true if argument is NIL" |
| `03`/`04`/`05` | `EQUAL[x;y]` | Appendix A, ст.57: "true if arguments are the same S-expression... uses eq on the atomic level and is recursive" | рекурсивний `equal_prim` helper, adversarially підтверджено (зламана версія реально провалює `03`/`05`) |
| `06`/`07` | `LIST[x1;...;xn]` | Appendix A, ст.57: "the value of list is a list of its arguments" | тривіально = `evlis` результат напряму |
| `08`/`09`/`10` | `AND[x1;...;xn]` | Appendix A, ст.58: FSUBR, "evaluated in sequence... until one is found that is false... value is false or true" | **не** останнє значення (як у Scheme/CL) -- лише T/NIL, точно за текстом. Short-circuit adversarially підтверджено |
| `11`/`12` | `OR[x1;...;xn]` | Appendix A, ст.58 | той самий short-circuit принцип |
| `13`/`14` | `NOT[x]` | Appendix A, ст.58 | "true if argument is false" |
| `15` | `RPLACA[x;y]` | Appendix A, ст.58: pseudo-function, деструктивна | guard: non-pointer (bit0==1) залишається неторканим, як і `car`/`cdr` |
| `16` | `RPLACD[x;y]` | Appendix A, ст.58 | той самий guard |

## Arithmetic (ст.63-64)

| Fixture | Функція | Провенанс | Примітка |
|---|---|---|---|
| `17` | `MINUS[x]` | Appendix A, ст.63 | унарна негація |
| `18`/`19` | `ADD1`/`SUB1` | Appendix A, ст.64 | x±1 |
| `20`/`21` | `MAX`/`MIN` | Appendix A, ст.64: FSUBR, indef. arity | n-арний fold; без визначеного identity-елемента (на відміну від PLUS/TIMES) -- перший аргумент є seed |
| `22`/`23` | `RECIP[x]` | Appendix A, ст.64: `[fixp[x]→0; T→quotient[1;x]]` | дослівна формула з тексту; для fixnum-only kernel дає 0 для \|x\|>1 автоматично |
| `24` | `QUOTIENT[x;y]` | Appendix A, ст.64 | `idiv`, truncation toward zero ("number theoretic quotient") |
| `25` | `REMAINDER[x;y]` | Appendix A, ст.64 | той самий `idiv`, залишок з `%rdx` |
| `26` | `DIVIDE[x;y]` | §4.2 ст.32: `divide[x;y] = cons[quotient[x;y];remainder[x;y]]` | один `idiv` дає обидві половини |
| `27`/`28` | `EXPT[x;y]` | Appendix A, ст.64 | ітеративне множення, `y≥0` (fixed-point case) |
| `29`/`30` | `LESSP[x;y]` | Appendix A, ст.64 | предикат |
| `31` | `GREATERP[x;y]` | Appendix A, ст.64 | предикат |
| `32`/`33` | `ONEP[x]` | Appendix A, ст.64 | fixed-point: точна рівність 1 (той самий тип narrowing, що й ZEROP у #22) |
| `34`/`35` | `MINUSP[x]` | Appendix A, ст.64 | предикат знаку |
| `36`/`37` | `NUMBERP[x]` | Appendix A, ст.64 | делегує на внутрішній `fixnump` |
| `38` | `FIXP[x]` | Appendix A, ст.64 | збігається з `NUMBERP` -- kernel лише fixed-point |
| `39` | `FLOATP[x]` | Appendix A, ст.64 | **завжди `NIL`** -- kernel не має floating-point типу взагалі; чесна, задокументована межа, не замовчана |
| `40` | `LOGOR` | Appendix A, ст.64 | n-арний fold, `or`, identity=0 |
| `41` | `LOGAND` | Appendix A, ст.64 | n-арний fold, `and`, identity=-1 (усі біти) |
| `42` | `LOGXOR` | Appendix A, ст.64 | n-арний fold, `xor`, identity=0 |
| `43`/`44` | `LEFTSHIFT[x;n]` | §4.2 ст.33: `x × 2^n`, від'ємний `n` зсуває вправо | `shl`/`sar` з variable count через `%cl` |

## Adversarial verification

`EQUAL` і `AND` (короткі замикання) перевірено не лише "проходить на
поточному коді": у копії `mccarthy-kernel.s` (поза git) тимчасово
зламано `equal_prim` (завжди повертає "не рівні") -- відповідні
fixtures `03`/`05` реально провалились, інші 42 не зачепило. Окремо
зламано `AND`'s перевірку хибності -- fixture `09` реально дав `T`
замість очікуваного `NIL`. Зламані копії видалено, трекований
`mccarthy-kernel.s` не змінювався під час перевірки.

## Verification

Повний `tests/closeout/run.sh` (усі корпуси + детермінованість)
перевірено після додавання цих 29 нових примітивів -- без регресій,
ISA-baseline (`tests/isa-baseline/`) підтверджує, що жоден новий
код не використовує gated-розширень (усе -- `idiv`/`imul`/`and`/
`or`/`xor`/`shl`/`sar`/`cmov`, baseline x86-64).
