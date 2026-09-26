# Відновлення пам'яті: free-storage list і reclamation cycle McCarthy 1960

Issue: [#65](https://github.com/juv4uk/mccarthy-eval/issues/65). Кожне
твердження позначено: `source-confirmed` (прямо читається в першоджерелі),
`reconstruction-derived` (сучасне рішення, потрібне для роботи на x86-64, не
приписується McCarthy), `empirically confirmed` (перевірено запуском; вказано
сходинку доказу).

## Першоджерело

J. McCarthy, *Recursive Functions of Symbolic Expressions and Their Computation
by Machine, Part I*, CACM, квітень 1960; Stanford author reprint
(`https://www-formal.stanford.edu/jmc/recursive.pdf`), розділ 4c
«Free-Storage List», с. 26–27, і авторська примітка 7 на с. 27.

`source-confirmed`:

- «The remaining registers (in our system the number, initially, is
  approximately 15,000) are arranged in a single list called the free-storage
  list. A certain register, FREE, in the program contains the location of the
  first register in this list.» (с. 26)
- «When a word is required to form some additional list structure, the first
  word on the free-storage list is taken and the number in register FREE is
  changed to become the location of the second word on the free-storage
  list.» (с. 26)
- «There is a fixed set of base registers in the program which contains the
  locations of list structures that are accessible to the program.» (с. 26)
- «Nothing happens until the program runs out of free storage. When a free
  register is wanted, and there is none left on the free-storage list, a
  reclamation cycle starts.» (с. 27)
- «First, the program finds all registers accessible from the base registers
  and makes their signs negative. [...] If the program encounters a register
  in this process which already has a negative sign, it assumes that this
  register has already been reached.» (с. 27)
- «After all of the accessible registers have had their signs changed, the
  program goes through the area of memory reserved for the storage of list
  structures and puts all the registers whose signs were not changed in the
  previous step back on the free-storage list, and makes the signs of the
  accessible registers positive again.» (с. 27)
- «[...] the reclamation process requires several seconds to execute [...]»
  (с. 27)
- Примітка 7: «We already called this process "garbage collection", but I
  guess I chickened out of using it in the paper».

## Відповідність IBM 704 → x86-64 / Intel Core i5-6400

| McCarthy 1960 | `mccarthy-kernel.s` | статус |
|---|---|---|
| регістр `FREE`, перший вільний регістр | `free_list`; `cons` бере перший, `FREE` ← наступний (посилання в `cdr`) | `source-confirmed` (алгоритм), розміщення посилання в `cdr` — `reconstruction-derived` |
| ~15 000 вільних регістрів | `HEAP_CELLS` = 2 097 152 комірок по 16 Б (32 МіБ) | `reconstruction-derived`: бітова карта позначок = 256 КіБ = рівно L2 одного ядра Skylake |
| цикл лише коли free-list порожній | `cons` → `reclaim` лише коли `free_list` порожній і область заповнена | `source-confirmed` |
| «signs negative» | біт на комірку в `gc_markbits` | `reconstruction-derived`: старший біт слова тут зайнятий від'ємними fixnum (тег `11`), тож «знак» винесено в окрему карту; «повернути знаки в плюс» = очистити карту перед наступним циклом |
| base registers | регістри процесора (кладуться на стек у `cons`), увесь стек викликів `[rsp, stack_base)` і область `[gc_roots_begin, gc_roots_end)` у `.bss` | `reconstruction-derived`; сканування консервативне: слово з тегом `00`, що вказує в зайняту частину області, — досяжне |
| прохід усієї області, непозначені → free-list | `reclaim`: sweep від кінця до початку, щоб FREE ішов за зростанням адрес; вільна комірка має `car = 0` | `source-confirmed` (алгоритм) |
| (у статті немає) | якщо після циклу вільних немає → `CONDITION kind=HEAP-EXHAUSTED`, код виходу 3 | `reconstruction-derived`: fail-closed замість мовчазної корупції |
| (у статті немає) | позначення явним стеком `gc_markstack`, не рекурсією | `reconstruction-derived`: глибокі списки не переповнюють стек процесора |

## Інші межі ресурсів (fail-closed), issue #65

`reconstruction-derived`: до цієї зміни ядро не перевіряло меж і в разі
переповнення мовчки давало хибні відповіді, падало або друкувало нескінченно.
Тепер кожна межа — названа умова в stderr, stdout скидається (`fflush`), код
виходу 3:

| межа | умова |
|---|---|
| область cons (живі дані більші за область) | `HEAP-EXHAUSTED` |
| push-down list (стек процесора) | `STACK-EXHAUSTED` — перевірка на вході в `eval`; м'яка межа стеку на старті піднімається до 1 ГіБ, якщо жорстка межа дозволяє |
| таблиця символів (65 536) | `SYMTAB-FULL` |
| пам'ять імен символів (1 МіБ) | `STRHEAP-FULL` |
| довжина токена (255 символів) | `TOKEN-TOO-LONG` (раніше мовчки обрізався) |
| кінець вводу посеред форми | `UNEXPECTED-EOF` (раніше нескінченна рекурсія → segfault) |
| зайва `)` | `UNEXPECTED-CLOSE` (раніше нескінченний друк `NIL`) |
| вихідний файл ≥ 16 МіБ | `SOURCE-TOO-LARGE` (раніше мовчки обрізався на 65 535 Б) |

`MCCARTHY_GC_STATS=1` друкує в stderr наприкінці: кількість циклів, скільки
регістрів повернуто в FREE останнім циклом, використано / ємність.

## Докази

`empirically confirmed`, сходинка — локальний запуск (i5-6400, WSL2, gcc 16.1 з
Guix, `qemu-x86_64` 10.2.1), 2026-09-26:

| перевірка | результат |
|---|---|
| усі попередні набори `tests/*/run.sh` | 141/141, без змін |
| `tests/historical-reclamation/run.sh` | 10/10 |
| 5 000 випадкових програм проти незалежного еталона `reference_mccarthy_1960.py`, в одному процесі | 5 000/5 000, 0 розбіжностей (до зміни — segfault / нескінченний друк) |
| те саме на збірці з областю 4 096 комірок | 5 000/5 000, **261 цикл** reclamation |
| 20 000 програм в одному процесі (робоча збірка) | 20 000/20 000, 1 цикл повернув 2 096 946 з 2 097 152 комірок |
| тривалість одного циклу на 2 097 152 комірках | ≈ 150 мс (мінімум із 7 запусків, різниця із збіркою без збирання; машина навантажена іншими процесами — оцінка). У статті: «several seconds» на IBM 704 для ~15 000 регістрів |
| `tests/closeout/run.sh` | QEMU/Conroe і детермінізм — див. PR |

## Відомі межі (не виправлено тут)

- `eval` шукає змінні в a-list, що росте з глибиною рекурсії — час O(глибина²).
  Це властивість самого алгоритму 1960 року: рекурсія глибиною 70 000 працює
  правильно, але хвилини. `source-confirmed` щодо алгоритму `assoc`.
- Сканування коренів консервативне: випадкове слово, схоже на вказівник,
  може утримати сміття до наступного циклу (не впливає на правильність).

---

# Reclamation: free-storage list and reclamation cycle, McCarthy 1960 (English mirror)

Issue #65. Primary source: McCarthy, CACM April 1960, Stanford reprint §4c
"Free-Storage List", pp. 26–27, footnote 7 ("We already called this process
'garbage collection'…"). The kernel now allocates from `free_list` (FREE) and,
only when free storage runs out, runs one reclamation cycle: mark every cell
reachable from the base registers, then sweep the area returning unmarked cells
to FREE (source-confirmed). Reconstruction-derived choices for x86-64: the
"sign" is one bit per cell in `gc_markbits` (the word's top bit is taken by
negative fixnums); base registers are the CPU registers, the whole native stack
and the `[gc_roots_begin, gc_roots_end)` globals, scanned conservatively; an
explicit mark stack avoids native recursion; `HEAP_CELLS` = 2,097,152 so the
mark bitmap is exactly 256 KiB — one Skylake core's L2. Every resource limit is
now a named fail-closed CONDITION with exit code 3 (HEAP-EXHAUSTED,
STACK-EXHAUSTED, SYMTAB-FULL, STRHEAP-FULL, TOKEN-TOO-LONG, UNEXPECTED-EOF,
UNEXPECTED-CLOSE, SOURCE-TOO-LARGE) instead of silent wrong answers, crashes or
endless output. Evidence (local run): 141/141 existing tests, 10/10 new tests,
5,000/5,000 random programs vs an independent Python reference in one process,
261 reclamation cycles on a 4,096-cell stress build with identical answers, one
cycle over 2,097,152 cells ≈ 150 ms on the i5-6400 (the paper: "several seconds"
on the IBM 704 for ~15,000 registers).
