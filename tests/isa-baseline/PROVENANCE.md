# Provenance: ISA-baseline witness (issue #7, "Фізичне виконання")

Ця перевірка доводить один пункт acceptance criteria issue #7
(RECONSTRUCTION-1960): "Correctness path should prefer scalar x86-64
and must not require AVX2/BMI2 or other optional extensions" -- не
декларативно, а через реальне виконання.

## Авторитетне джерело ISA-каталогу

Список мнемонік для кожного gated-розширення взято дослівно з
власного ISA-каталогу `my-lisp` -- **не з власного здогаду цієї
сесії**. Це фізичний реєстр інструкцій, який `my-lisp` вже підтримує
для свого власного machine-layer (`lib/machine/isa/`), і саме він тут
є authority, а не окрема регексна евристика.

Джерело: `juv4uk/my-lisp`, знімок 2026-09-22.

| Файл | Blob SHA |
|---|---|
| `lib/machine/cpu/intel-core-i5-6400.lisp` | `1b4f98e86f8e9fac14dcc74862bf82afd536c8a3` |
| `lib/machine/isa/aes-ni.lisp` | `388928a9ee78f38bab13e61e961af0d184c6697c` |
| `lib/machine/isa/pclmulqdq.lisp` | `dc7698d3aeeae9b591f3ea6bbe380c3d7461bf10` |
| `lib/machine/isa/avx.lisp` | `eb908afc48744a4352a7ef2adf6c87447a21c35e` |
| `lib/machine/isa/f16c.lisp` | `16f8130244905d595819697eab452736f4c0c876` |
| `lib/machine/isa/fma3.lisp` | `86795e9751d26b0cfe8cec74b9f4fa4d796a7aac` |
| `lib/machine/isa/bmi1.lisp` | `b2c615fabddfab9b527df3d62f7984b05d7bf22d` |
| `lib/machine/isa/bmi2.lisp` | `f94ea0eb10509b04123dbce6b4301306e2191173` |
| `lib/machine/isa/avx2.lisp` | `ebef5179e1a09498b935fa8ad6822e910fda057e` |
| `lib/machine/isa/rdrand.lisp` | `ea74c5ebfd94a2f3e2b3af6a87eeb33f99ea70a6` |
| `lib/machine/isa/rdseed.lisp` | `9bb853a43b8ed89188f72e8fb53471c046ede9fa` |
| `lib/machine/isa/adx.lisp` | `fd17a7e2c4d0d173ff02b929de7d1ecaa44dbe4e` |
| `lib/machine/isa/xsave.lisp` | `bffda21d28c5fbb86879605851969567b859dedf` |
| `lib/machine/isa/clflushopt.lisp` | `c62e93d322ce647c44c248bac8a70cbff7363110` |

Кожен файл сам себе позначає `(coverage seed)` або `(coverage
identity-seed)` -- каталог не претендує на вичерпний список усіх
інструкцій розширення, лише на приклад-ідентифікатор. Тому перевірка
тут навмисно доповнена структурним пошуком (будь-яка VEX-кодована
мнемоніка, будь-який xmm/ymm/zmm регістр) -- це покриває цілі родини
AVX/AVX2/FMA3/F16C незалежно від того, які саме приклад-інструкції
занесені до "seed"-каталогу.

Фізичний профіль самого CPU (`intel-core-i5-6400.lisp`) підтверджено
раніше цієї ж сесії проти живої машини -- див. окрему звірку
`docs/references/hardware/OWNER-HARDWARE-PROFILE.md` vs. `/proc/
cpuinfo` (точний збіг, включно з family/model/stepping і повним
списком gated/unavailable розширень).

## Два незалежні докази, обидва emпirically confirmed

1. **Статичний** -- `objdump -d` реального `mccarthy-kernel` не
   містить жодної мнемоніки з таблиці вище, жодної VEX-кодованої
   інструкції, жодного xmm/ymm/zmm регістра. 990 інструкцій усього,
   повний унікальний набір мнемонік: `add and call cmp cs data16
   endbr64 hlt imul inc ja jb je jge jl jmp jne lea mov movzx neg nop
   or pop push ret sar sete shl shr sub test xchg xor` -- чистий
   скалярний x86-64 baseline.
2. **Динамічний, найсильніший** -- увесь `tests/historical-core/`
   корпус (21 fixture, issue #9) запущено під `qemu-x86_64 -cpu
   Conroe` (Intel Core 2, 2006 р., без SSE4.x/AVX/BMI/AES-NI
   **фізично**, не лише "не використано"). 21/21 дали той самий
   результат, що на реальному i5-6400. Це не "у коді не видно
   інструкції" -- це "код справді виконується без апаратної
   спроможності".

Обидва докази відтворювані одним викликом:
`tests/isa-baseline/check-isa-baseline.sh`.

---

## English mirror

This proves one acceptance criterion of issue #7: the historical-core
implementation is plain scalar x86-64 and requires none of this CPU's
gated extensions. The per-extension mnemonic list is taken verbatim
from `my-lisp`'s own ISA catalogue (`lib/machine/isa/*.lisp`, table
above with blob SHAs) -- not guessed by this session -- and widened
with a structural scan for any VEX-encoded mnemonic or xmm/ymm/zmm
register, since each catalogue file self-labels its coverage as a
non-exhaustive "seed". Two independent, empirically confirmed proofs:
a static disassembly scan (zero matches, zero VEX instructions, plain
scalar mnemonic set) and a dynamic one -- the full 21-fixture
historical-core corpus re-run under QEMU emulating a 2006 Intel Core 2
"Conroe" (no SSE4.x/AVX/BMI/AES-NI at all), with all 21 matching the
real i5-6400's output exactly. Reproduce both with
`tests/isa-baseline/check-isa-baseline.sh`.
