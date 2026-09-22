# Provenance: CONDITION diagnostic для незв'язаного виклику (2026-09-22)

**Це не historical McCarthy 1960/1962 семантика.** Жодне первинне
джерело цієї реконструкції не описує error-reporting для виклику
незв'язаної функції. Це навмисно позначена як **сучасна
runtime-зручність**, аналогічно до `startup.lisp`'s auto-load
механізму, і так само не претендує на historical authority.

## Що саме змінилось, і що -- ні

`plain_call` (гілка `eval`, що трактує виклик функції за іменем) при
`assoc`-not-found тепер друкує один рядок у **stderr**:

```
CONDITION kind=UNBOUND name=<символ>
```

**Значення, яке повертається програмі, не змінилось** -- і досі
`NIL`, байт-в-байт те саме, що й до цієї зміни (fix issue: "unbound
function call no longer crashes", той самий session). stdout -- те,
проти чого звіряються всі `.expected`-файли в `tests/historical-core/`
та інших корпусах -- також не змінився: `tests/historical-core/
22-plain-call-unbound-function-no-crash.lisp` і далі дає `NIL` на
stdout, підтверджено окремим прогоном (`2>/dev/null`).

## Джерело формату (не коду)

Формат `CONDITION kind=...` -- натхнення з реального REPL-транскрипту
`wsm-os-lisp` (`artifacts/m5h-repl-fixture-qemu-serial-transcript.txt`,
той самий цикл-екосистема, окремий репозиторій, WSM's власний bare-metal
REPL, Rust): `WSM-OS CONDITION schema=1 kind=TYPE source=repl value=foo`.
Використано лише сама ІДЕЯ (структурований, розпізнаваний рядок
замість тихого значення, без переривання REPL-циклу) -- **жодного
коду не скопійовано й не портовано**; цей файл лишається чистим,
рукописним асемблером, без жодної Rust-залежності.

## Fixtures

Runner (`run.sh`) перевіряє **stdout і stderr окремо** -- це і є суть
тесту: `01`/`02` доводять, що stdout лишається `NIL` (стара семантика
незмінна), а stderr несе нову діагностику з правильним іменем символу.

| Fixture | Виклик | stdout (незмінно) | stderr (нове) |
|---|---|---|---|
| `01-unbound-call-condition` | `(NOTAFUNCTION 1 2 3)` | `NIL` | `CONDITION kind=UNBOUND name=NOTAFUNCTION` |
| `02-env-condition` | `(ENV)` -- історичний приклад, той самий, що спричинив segfault до фіксу | `NIL` | `CONDITION kind=UNBOUND name=ENV` |

## Adversarial verification

У НЕ закомічену копію `mccarthy-kernel.s` видалено `mov $NIL_SYM,
%rax` (перезавантаження результату після `fprintf`, який сам по собі
псує `%rax`) -- реально дало сміттєве значення (`LAMBDA`, залишок
регістра з попереднього виклику) замість `NIL` на stdout. Це доводить,
що перезавантаження в коді -- не косметика, а те, що безпосередньо
зберігає стару семантику незмінною після додавання діагностики. Копію
видалено, трекований файл не змінювався під час перевірки.
