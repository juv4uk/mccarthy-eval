/* mccarthy-kernel.s -- full McCarthy 1960 kernel: reader, dynamic
 * symbol table, complete eval (QUOTE/ATOM/EQ/COND/CAR/CDR/CONS/
 * LABEL/LAMBDA + plain function calls), general printer, and a
 * minimal top-level driver with a DEFINE form for a persistent
 * global environment. x86_64 (System V ABI), GNU assembler.
 *
 * This is the "cross the threshold" milestone from
 * docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md: unlike
 * mccarthy-eval.s (two hand-built traces, fixed 9-symbol table),
 * this reads real Lisp SOURCE TEXT and evaluates arbitrary programs
 * written in it, including user-defined recursive functions.
 *
 * Representation: same tagged word as mccarthy-eval.s (bit0=1 ->
 * symbol, bit0=0 -> cons pointer), but symbols are now INTERNED AT
 * RUNTIME from source text, not compile-time constants.
 */

    .section .rodata
fmt_str:
    .asciz "%s"
fmt_int:
    .asciz "%ld"
fmt_space:
    .asciz " "
fmt_dot:
    .asciz " . "
fmt_open:
    .asciz "("
fmt_close:
    .asciz ")"
fmt_newline:
    .asciz "\n"
fmt_mode_r:
    .asciz "r"
fmt_prompt:
    .asciz "> "
fname_startup:
    .asciz "startup.lisp"

sym_NIL:    .asciz "NIL"
sym_T:      .asciz "T"

    .section .bss
    .align 16
heap:                       /* cons-cell heap, bump-allocated */
    .skip 16 * 65536
heap_ptr:
    .skip 8

    .align 16
strheap:                    /* permanent storage for interned symbol names */
    .skip 1 * 1048576
strheap_ptr:
    .skip 8

symtab:                     /* array of pointers to interned name strings */
    .skip 8 * 65536
symtab_count:
    .skip 8

tokbuf:                     /* scratch buffer for the token currently being read */
    .skip 256

input_ptr:                  /* reader's current position in the source buffer */
    .skip 8

filebuf:                    /* loaded contents of the argv[1] .lisp file */
    .skip 65536

linebuf:                    /* one line of REPL input at a time */
    .skip 4096

argc_saved:
    .skip 8

global_env:
    .skip 8

    .text
    .globl main

/* Tag scheme, extended for numbers (2026-08-28, next step):
 *   bits[1:0] == 00  -> cons pointer (16-byte aligned heap address)
 *   bits[1:0] == 01  -> symbol   (payload = value >> 2)
 *   bits[1:0] == 11  -> fixnum   (payload = value >> 2, arithmetic shift)
 * atomp (bit0 test) still correctly means "not a cons pointer" for
 * BOTH symbols and fixnums, unchanged everywhere it's already used.
 * Symbol tags are now id*4|1 (was id*2|1) so bit1 stays 0 for them. */
.equ NIL_SYM, 1              /* guaranteed by interning "NIL" first, at startup */
.equ T_SYM,   5               /* guaranteed by interning "T" second */

/* =================================================================
 * Primitives (unchanged in spirit from mccarthy-eval.s)
 * ================================================================= */
cons:
    mov     heap_ptr(%rip), %rax
    mov     %rdi, (%rax)
    mov     %rsi, 8(%rax)
    lea     16(%rax), %rdx
    mov     %rdx, heap_ptr(%rip)
    ret

/* car(rdi=val) -> rax. З перевіркою типу: будь-який не-вказівник
 * (bit0==1, тобто atomp -- покриває обидва immediate-теги цього
 * kernel-а, символи й fixnum-и) повертає NIL замість розіменування
 * rdi як адреси пам'яті. Виправляє клас краху "невірна арність",
 * задокументований у README.md ("Three real crashes... 3. Wrong
 * arity") -- caddr[e] на закороткому списку діставався car(NIL_SYM),
 * розіменовуючи сирі тег-біти (крихітне непарне число) як вказівник.
 * Узгоджується з наявною філософією тихого fallback-у на NIL цього
 * kernel-а (вже вжитою для незв'язаних символів в assoc), не нова
 * концепція. */
car:
    mov     %rdi, %rax
    and     $1, %rax
    jnz     .car_not_pointer
    mov     (%rdi), %rax
    ret
.car_not_pointer:
    mov     $NIL_SYM, %rax
    ret

/* cdr(rdi=val) -> rax. Та сама перевірка типу, що й у car вище. */
cdr:
    mov     %rdi, %rax
    and     $1, %rax
    jnz     .cdr_not_pointer
    mov     8(%rdi), %rax
    ret
.cdr_not_pointer:
    mov     $NIL_SYM, %rax
    ret

atomp:
    mov     %rdi, %rax
    and     $1, %rax
    ret

eq_prim:
    xor     %eax, %eax
    cmp     %rsi, %rdi
    sete    %al
    ret

/* fixnump(rdi=val) -> rax (1/0): bits[1:0] == 11 */
fixnump:
    mov     %rdi, %rax
    and     $3, %rax
    cmp     $3, %rax
    sete    %al
    movzx   %al, %eax
    ret

/* mkfix(rdi=n) -> rax = tagged fixnum */
mkfix:
    mov     %rdi, %rax
    shl     $2, %rax
    or      $3, %rax
    ret

/* getfix(rdi=tagged fixnum) -> rax = signed integer, sign-preserving */
getfix:
    mov     %rdi, %rax
    sar     $2, %rax
    ret

cadr:
    call    cdr
    mov     %rax, %rdi
    jmp     car

caddr:
    call    cdr
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

caar:
    call    car
    mov     %rax, %rdi
    jmp     car

cadar:
    call    car
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

caddar:
    call    car
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

assoc:
.assoc_loop:
    cmp     $NIL_SYM, %rsi
    je      .assoc_nil
    mov     (%rsi), %rdx
    mov     (%rdx), %rcx
    cmp     %rcx, %rdi
    je      .assoc_found
    mov     8(%rsi), %rsi
    jmp     .assoc_loop
.assoc_found:
    mov     8(%rdx), %rax
    ret
.assoc_nil:
    mov     $NIL_SYM, %rax
    ret

pair:
    push    %r12
    push    %r13
    push    %rbx
    mov     %rdi, %r12
    mov     %rsi, %r13
    cmp     $NIL_SYM, %r12
    je      .pair_nil
    mov     %r12, %rdi
    call    car
    mov     %rax, %rbx
    mov     %r13, %rdi
    call    car
    mov     %rbx, %rdi
    mov     %rax, %rsi
    call    cons
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %r12
    mov     %r13, %rdi
    call    cdr
    mov     %rax, %r13
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    pair
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .pair_done
.pair_nil:
    mov     $NIL_SYM, %rax
.pair_done:
    pop     %rbx
    pop     %r13
    pop     %r12
    ret

append:
    push    %r12
    push    %r13
    mov     %rdi, %r12
    mov     %rsi, %r13
    cmp     $NIL_SYM, %r12
    je      .append_base
    mov     %r12, %rdi
    call    car
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    append
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .append_done
.append_base:
    mov     %r13, %rax
.append_done:
    pop     %r13
    pop     %r12
    ret

evlis:
    push    %r12
    push    %r13
    mov     %rdi, %r12
    mov     %rsi, %r13
    cmp     $NIL_SYM, %r12
    je      .evlis_base
    mov     %r12, %rdi
    call    car
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .evlis_done
.evlis_base:
    mov     $NIL_SYM, %rax
.evlis_done:
    pop     %r13
    pop     %r12
    ret

/* evcon[c;a] = [eval[caar[c];a] -> eval[cadar[c];a]; T -> evcon[cdr[c];a]]
 * Deviation from the 1960 paper, noted honestly: if no clause matches
 * (c reaches NIL), McCarthy leaves this undefined; we return NIL as a
 * safe fallback rather than leaving it truly undefined (a documented
 * simplification, not a silently different semantics). */
evcon:
    push    %r12
    push    %r13
    mov     %rdi, %r12          /* c */
    mov     %rsi, %r13          /* a */
    cmp     $NIL_SYM, %r12
    je      .evcon_nomatch
    mov     %r12, %rdi
    call    caar
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    je      .evcon_next
    mov     %r12, %rdi
    call    cadar
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    jmp     .evcon_done
.evcon_next:
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evcon
    jmp     .evcon_done
.evcon_nomatch:
    mov     $NIL_SYM, %rax
.evcon_done:
    pop     %r13
    pop     %r12
    ret

/* appq[m] = [null[m] -> NIL; T -> cons[list[QUOTE;car[m]]; appq[cdr[m]]]]
 * Exactly the 1960 paper's own appq, used by apply -- see
 * mccarthy-1960-eval-apply-primary-source-2026-08-28.md. Needed here
 * for the same reason it exists in the original: plain_call's evlis
 * already evaluated these values once; feeding them back into eval
 * unquoted would evaluate them a SECOND time as if they were code
 * (e.g. the realized list (A) would be evaluated as "call A with no
 * args", not treated as the data value (A)). Found this the hard way,
 * via a real segfault, not by remembering the paper. */
appq:
    push    %r12
    mov     %rdi, %r12
    cmp     $NIL_SYM, %r12
    je      .appq_nil
    mov     %r12, %rdi
    call    car
    mov     %rax, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                /* (car[m]) */
    mov     $QUOTE_SYM, %rdi
    mov     %rax, %rsi
    call    cons                /* (QUOTE car[m]) */
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    call    appq
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .appq_done
.appq_nil:
    mov     $NIL_SYM, %rax
.appq_done:
    pop     %r12
    ret

/* =================================================================
 * eval(rdi=e, rsi=a) -> rax -- now complete: QUOTE/ATOM/EQ/COND/CAR/
 * CDR/CONS/LABEL/LAMBDA/plain-call, matching the 1960 paper in full.
 * ================================================================= */
eval:
    push    %r12
    push    %r13
    push    %r14
    mov     %rdi, %r12
    mov     %rsi, %r13

    mov     %r12, %rdi
    call    atomp
    test    %rax, %rax
    jz      .not_atom
    /* T self-evaluates, matching how NIL already, implicitly,
     * self-evaluates via assoc's not-found fallback. Without this,
     * T never appears as a value -- every COND T-clause silently
     * fails, and every "truthy" comparison silently reads as false. */
    cmp     $T_SYM, %r12
    jne     .not_t
    mov     $T_SYM, %rax
    jmp     .eval_done
.not_t:
    /* fixnums self-evaluate too, same reasoning as T above -- a
     * literal number is never meant to be looked up in the
     * environment, it already IS its own value. */
    mov     %r12, %rdi
    call    fixnump
    test    %rax, %rax
    jz      .not_fixnum
    mov     %r12, %rax
    jmp     .eval_done
.not_fixnum:
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    assoc
    jmp     .eval_done

.not_atom:
    mov     %r12, %rdi
    call    car
    mov     %rax, %r14
    mov     %r14, %rdi
    call    atomp
    test    %rax, %rax
    jz      .head_not_atom

    cmp     $QUOTE_SYM, %r14
    jne     .try_atom
    mov     %r12, %rdi
    call    cadr
    jmp     .eval_done

.try_atom:
    cmp     $ATOM_SYM, %r14
    jne     .try_eq
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    atomp
    test    %rax, %rax
    jz      .atom_false
    mov     $T_SYM, %rax
    jmp     .eval_done
.atom_false:
    mov     $NIL_SYM, %rax
    jmp     .eval_done

.try_eq:
    cmp     $EQ_SYM, %r14
    jne     .try_cond
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    push    %rax
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rsi
    pop     %rdi
    call    eq_prim
    test    %rax, %rax
    jz      .eq_false
    mov     $T_SYM, %rax
    jmp     .eval_done
.eq_false:
    mov     $NIL_SYM, %rax
    jmp     .eval_done

.try_cond:
    cmp     $COND_SYM, %r14
    jne     .try_car
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evcon
    jmp     .eval_done

.try_car:
    cmp     $CAR_SYM, %r14
    jne     .try_cdr
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    car
    jmp     .eval_done

.try_cdr:
    cmp     $CDR_SYM, %r14
    jne     .try_cons
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    cdr
    jmp     .eval_done

.try_cons:
    cmp     $CONS_SYM, %r14
    jne     .try_zerop
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    push    %rax
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .eval_done

/* Arithmetic, added 2026-08-28 -- not in McCarthy 1960's own seven,
 * bolted on exactly the way every real Lisp bolts on numbers: as
 * ordinary extra dispatch branches over a new fixnum tag, nothing
 * about QUOTE/CAR/CDR/CONS/ATOM/EQ/COND/LABEL/LAMBDA changed. */
.try_zerop:
    cmp     $ZEROP_SYM, %r14
    jne     .try_times
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    test    %rax, %rax
    jz      .zerop_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.zerop_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

/* times[x1;...;xn] -- LISP 1.5 Programmer's Manual (1962), SS4.2 p.32:
 * "is a function of any number of arguments, whose value is the
 * product (with correct sign) of its arguments." Real n-ary fold, not
 * the earlier hard-coded 2-argument version -- source-faithful now,
 * not a documented narrowing (issue #27). evlis evaluates every
 * argument first (matches evlis[cdr[e];a] used by every other
 * multi-arg form in this evaluator); the fold itself needs no
 * environment lookups, so a plain loop over already-evaluated fixnums
 * is enough. n=2 reduces to exactly the previous fixed-arity behavior. */
.try_times:
    cmp     $TIMES_SYM, %r14
    jne     .try_difference
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     $1, %rax
    push    %rax
.times_loop:
    cmp     $NIL_SYM, %rbx
    je      .times_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    imul    %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .times_loop
.times_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

.try_difference:
    cmp     $DIFFERENCE_SYM, %r14
    jne     .try_plus
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    push    %rax
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    mov     %rcx, %rdi
    sub     %rax, %rdi
    call    mkfix
    jmp     .eval_done

/* plus[x1;...;xn] -- LISP 1.5 Programmer's Manual (1962), SS4.2 p.31:
 * "is a function of any number of arguments whose value is the
 * algebraic sum of the arguments." Same n-ary fold as times above,
 * source-faithful now (issue #27); n=2 reduces to the previous
 * fixed-arity behavior exactly. */
.try_plus:
    cmp     $PLUS_SYM, %r14
    jne     .plain_call
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     $0, %rax
    push    %rax
.plus_loop:
    cmp     $NIL_SYM, %rbx
    je      .plus_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    add     %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .plus_loop
.plus_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

.plain_call:
    /* T -> eval[cons[assoc[car[e];a]; appq[evlis[cdr[e];a]]]; a]
     * (the appq[...] is the fix above -- 1960 paper's apply does
     * this too, cons[f;appq[args]], for exactly this reason) */
    mov     %r14, %rdi
    mov     %r13, %rsi
    call    assoc
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rdi
    call    appq
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    jmp     .eval_done

.head_not_atom:
    mov     %r12, %rdi
    call    caar
    cmp     $LABEL_SYM, %rax
    jne     .try_lambda
    /* eval[cons[caddar[e];cdr[e]]; cons[cons[cadar[e];car[e]];a]]
     * (uses a dotted (name . form) binding, matching this system's
     * own assoc/pair convention throughout -- not the literal
     * list[...] in the 1960 text, which would make assoc's plain
     * cdr-return inconsistent with every other binding in the
     * system. A deliberate, noted fix, not a silent deviation.) */
    mov     %r12, %rdi
    call    cadar               /* name */
    push    %rax
    mov     %r12, %rdi
    call    car                 /* car[e] = (LABEL name (LAMBDA...)) */
    pop     %rdi                /* name */
    mov     %rax, %rsi
    call    cons                /* (name . car[e]) */
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    cons                /* ((name . car[e]) . a) */
    mov     %rax, %r13          /* new_env */

    mov     %r12, %rdi
    call    caddar              /* (LAMBDA params body) */
    push    %rax
    mov     %r12, %rdi
    call    cdr                 /* raw, unevaluated argument expressions */
    mov     %rax, %rsi
    pop     %rdi
    call    cons                /* ((LAMBDA params body) arg1 arg2 ...) */
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    jmp     .eval_done

.try_lambda:
    cmp     $LAMBDA_SYM, %rax
    jne     .eval_done          /* head is neither LABEL nor LAMBDA: give up cleanly */
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    push    %rax
    mov     %r12, %rdi
    call    cadar
    mov     %rax, %rdi
    pop     %rsi
    call    pair
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    append
    push    %rax
    mov     %r12, %rdi
    call    caddar
    mov     %rax, %rdi
    pop     %rsi
    call    eval

.eval_done:
    pop     %r14
    pop     %r13
    pop     %r12
    ret

/* =================================================================
 * Dynamic symbol table
 * ================================================================= */

/* strcmp_eq(rdi=a, rsi=b) -> rax (1/0), both null-terminated */
strcmp_eq:
.strcmp_loop:
    movzx   (%rdi), %eax
    movzx   (%rsi), %edx
    cmp     %eax, %edx
    jne     .strcmp_ne
    test    %al, %al
    jz      .strcmp_eq_yes
    inc     %rdi
    inc     %rsi
    jmp     .strcmp_loop
.strcmp_ne:
    xor     %eax, %eax
    ret
.strcmp_eq_yes:
    mov     $1, %eax
    ret

/* strlen_z(rdi=s) -> rax */
strlen_z:
    xor     %rax, %rax
.strlen_loop:
    cmpb    $0, (%rdi,%rax)
    je      .strlen_done
    inc     %rax
    jmp     .strlen_loop
.strlen_done:
    ret

/* intern(rdi=name, null-terminated) -> rax = tagged symbol value */
intern:
    push    %r12
    push    %r13
    mov     %rdi, %r12          /* name to look up */
    xor     %r13, %r13          /* index i = 0 */
.intern_loop:
    cmp     symtab_count(%rip), %r13
    jge     .intern_new
    mov     symtab(,%r13,8), %rdi
    mov     %r12, %rsi
    call    strcmp_eq
    test    %rax, %rax
    jnz     .intern_found
    inc     %r13
    jmp     .intern_loop
.intern_found:
    lea     0(,%r13,4), %rax     /* id*4 -- leaves bit1=0, marking "symbol" */
    or      $1, %rax
    jmp     .intern_done
.intern_new:
    /* copy name into permanent string heap */
    mov     %r12, %rdi
    call    strlen_z
    mov     %rax, %rdx           /* len */
    mov     strheap_ptr(%rip), %rdi
    mov     %rdi, %rcx           /* save dest start */
    xor     %rax, %rax
.intern_copy:
    cmp     %rax, %rdx
    je      .intern_copy_done
    movb    (%r12,%rax,1), %r8b
    movb    %r8b, (%rdi,%rax,1)
    inc     %rax
    jmp     .intern_copy
.intern_copy_done:
    movb    $0, (%rdi,%rdx,1)    /* null terminator */
    lea     1(%rdi,%rdx,1), %rax
    mov     %rax, strheap_ptr(%rip)
    mov     symtab_count(%rip), %r13
    mov     %rcx, symtab(,%r13,8)
    lea     0(,%r13,4), %rax     /* id*4 -- leaves bit1=0, marking "symbol" */
    or      $1, %rax
    push    %rax
    inc     %r13
    mov     %r13, symtab_count(%rip)
    pop     %rax
.intern_done:
    pop     %r13
    pop     %r12
    ret

/* =================================================================
 * Reader: text -> S-expression, using the global input_ptr cursor
 * ================================================================= */

/* is_delim(al) -> sets ZF if al is space/tab/newline/'('/')'/0 */
skip_ws:
    mov     input_ptr(%rip), %rdi
.skip_ws_loop:
    movzx   (%rdi), %eax
    cmp     $' ', %al
    je      .skip_ws_adv
    cmp     $'\t', %al
    je      .skip_ws_adv
    cmp     $'\n', %al
    je      .skip_ws_adv
    jmp     .skip_ws_done
.skip_ws_adv:
    inc     %rdi
    jmp     .skip_ws_loop
.skip_ws_done:
    mov     %rdi, input_ptr(%rip)
    ret

read_atom:
    call    skip_ws
    mov     input_ptr(%rip), %rsi
    lea     tokbuf(%rip), %rdi
.read_atom_loop:
    movzx   (%rsi), %eax
    test    %al, %al
    jz      .read_atom_end
    cmp     $' ', %al
    je      .read_atom_end
    cmp     $'\t', %al
    je      .read_atom_end
    cmp     $'\n', %al
    je      .read_atom_end
    cmp     $'(', %al
    je      .read_atom_end
    cmp     $')', %al
    je      .read_atom_end
    movb    %al, (%rdi)
    inc     %rdi
    inc     %rsi
    jmp     .read_atom_loop
.read_atom_end:
    movb    $0, (%rdi)
    mov     %rsi, input_ptr(%rip)
    lea     tokbuf(%rip), %rdi
    call    looks_numeric
    test    %rax, %rax
    jz      .read_atom_symbol
    lea     tokbuf(%rip), %rdi
    call    parse_int
    ret
.read_atom_symbol:
    lea     tokbuf(%rip), %rdi
    call    intern
    ret

/* looks_numeric(rdi=tokbuf, null-terminated) -> rax (1/0):
 * a digit, or '-' followed immediately by a digit. */
/* looks_numeric(rdi=tokbuf, null-terminated) -> rax (1/0). Scans the
 * WHOLE token, not just the first character -- a token like "5.124"
 * starts with a digit but is not a valid integer, and this kernel has
 * no floating-point type at all. Getting this wrong doesn't error;
 * it silently mis-parses the '.' as a "digit" via unsigned wraparound
 * in parse_int, producing garbage (empirically found: "5.124" and
 * "1266.156" summed to 13218280, not a decimal-point error message).
 * A token only counts as numeric if, after an optional leading '-',
 * every remaining character is '0'-'9' and there's at least one. */
looks_numeric:
    movzx   (%rdi), %eax
    cmp     $'-', %al
    jne     .ln_scan
    inc     %rdi
.ln_scan:
    xor     %r8, %r8             /* saw at least one digit? */
.ln_loop:
    movzx   (%rdi), %eax
    test    %al, %al
    jz      .ln_check_end
    cmp     $'0', %al
    jb      .ln_no
    cmp     $'9', %al
    ja      .ln_no
    mov     $1, %r8
    inc     %rdi
    jmp     .ln_loop
.ln_check_end:
    test    %r8, %r8
    jz      .ln_no
    mov     $1, %eax
    ret
.ln_no:
    xor     %eax, %eax
    ret

/* parse_int(rdi=tokbuf) -> rax = tagged fixnum */
parse_int:
    xor     %r8, %r8             /* negative flag */
    movzx   (%rdi), %eax
    cmp     $'-', %al
    jne     .pi_loop
    mov     $1, %r8
    inc     %rdi
.pi_loop:
    xor     %rsi, %rsi           /* accumulator */
.pi_digits:
    movzx   (%rdi), %eax
    test    %al, %al
    jz      .pi_done
    sub     $'0', %al
    movzx   %al, %rax
    imul    $10, %rsi, %rsi
    add     %rax, %rsi
    inc     %rdi
    jmp     .pi_digits
.pi_done:
    test    %r8, %r8
    jz      .pi_positive
    neg     %rsi
.pi_positive:
    mov     %rsi, %rdi
    call    mkfix
    ret

read_sexpr:
    call    skip_ws
    mov     input_ptr(%rip), %rdi
    movzx   (%rdi), %eax
    cmp     $'(', %al
    jne     .read_sexpr_atom
    inc     %rdi
    mov     %rdi, input_ptr(%rip)
    call    read_list
    ret
.read_sexpr_atom:
    call    read_atom
    ret

read_list:
    call    skip_ws
    mov     input_ptr(%rip), %rdi
    movzx   (%rdi), %eax
    cmp     $')', %al
    jne     .read_list_elem
    inc     %rdi
    mov     %rdi, input_ptr(%rip)
    mov     $NIL_SYM, %rax
    ret
.read_list_elem:
    call    read_sexpr
    push    %rax
    call    read_list
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    ret

/* =================================================================
 * Printer
 * ================================================================= */
print_sym_name:                 /* rdi = tagged symbol -> prints its name */
    mov     %rdi, %rax
    shr     $2, %rax
    mov     symtab(,%rax,8), %rsi
    lea     fmt_str(%rip), %rdi
    xor     %eax, %eax
    call    printf
    ret

print_fixnum:                   /* rdi = tagged fixnum -> prints its decimal value */
    call    getfix
    mov     %rax, %rsi
    lea     fmt_int(%rip), %rdi
    xor     %eax, %eax
    call    printf
    ret

print_sexpr:
    push    %rbx
    mov     %rdi, %rbx
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jz      .print_cons
    mov     %rbx, %rdi
    call    fixnump
    test    %rax, %rax
    jz      .print_symbol
    mov     %rbx, %rdi
    call    print_fixnum
    jmp     .print_done
.print_symbol:
    mov     %rbx, %rdi
    call    print_sym_name
    jmp     .print_done
.print_cons:
    lea     fmt_open(%rip), %rdi
    xor     %eax, %eax
    call    printf
    mov     (%rbx), %rdi
    call    print_sexpr
    mov     8(%rbx), %rbx
.print_tail_loop:
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jnz     .print_tail_end
    lea     fmt_space(%rip), %rdi
    xor     %eax, %eax
    call    printf
    mov     (%rbx), %rdi
    call    print_sexpr
    mov     8(%rbx), %rbx
    jmp     .print_tail_loop
.print_tail_end:
    cmp     $NIL_SYM, %rbx
    je      .print_close
    lea     fmt_dot(%rip), %rdi
    xor     %eax, %eax
    call    printf
    mov     %rbx, %rdi
    call    print_sexpr
.print_close:
    lea     fmt_close(%rip), %rdi
    xor     %eax, %eax
    call    printf
.print_done:
    pop     %rbx
    ret

/* =================================================================
 * Top-level driver: read forms from `program`, evaluate each with
 * an accumulating global_env, print results. (DEFINE name expr)
 * is handled here, not inside eval -- it is nothing but the same
 * cons+pair mechanism LAMBDA application already uses internally,
 * exposed as an explicit top-level operation, not new magic.
 * ================================================================= */
.equ QUOTE_SYM,  9
.equ ATOM_SYM,   13
.equ EQ_SYM,     17
.equ COND_SYM,   21
.equ CAR_SYM,    25
.equ CDR_SYM,    29
.equ CONS_SYM,   33
.equ LABEL_SYM,  37
.equ LAMBDA_SYM, 41
.equ DEFINE_SYM, 45
.equ ZEROP_SYM,      49
.equ TIMES_SYM,      53
.equ DIFFERENCE_SYM, 57
.equ PLUS_SYM,       61

    .text

main:
    push    %rbx
    mov     %rsi, %rbx           /* argv, saved before any other call clobbers rsi */
    mov     %rdi, argc_saved(%rip)  /* argc, saved to memory (keeps push-count/stack
                                        alignment simple -- an extra register push
                                        here would need a matching pad, this doesn't) */

    lea     heap(%rip), %rax
    mov     %rax, heap_ptr(%rip)
    lea     strheap(%rip), %rax
    mov     %rax, strheap_ptr(%rip)
    movq    $0, symtab_count(%rip)

    lea     sym_NIL(%rip), %rdi
    call    intern                /* guarantees NIL_SYM == 1 */
    lea     sym_T(%rip), %rdi
    call    intern                /* guarantees T_SYM == 3 */
    lea     sym_QUOTE(%rip), %rdi
    call    intern
    lea     sym_ATOM(%rip), %rdi
    call    intern
    lea     sym_EQ(%rip), %rdi
    call    intern
    lea     sym_COND(%rip), %rdi
    call    intern
    lea     sym_CAR(%rip), %rdi
    call    intern
    lea     sym_CDR(%rip), %rdi
    call    intern
    lea     sym_CONS(%rip), %rdi
    call    intern
    lea     sym_LABEL(%rip), %rdi
    call    intern
    lea     sym_LAMBDA(%rip), %rdi
    call    intern
    lea     sym_DEFINE(%rip), %rdi
    call    intern
    lea     sym_ZEROP(%rip), %rdi
    call    intern
    lea     sym_TIMES(%rip), %rdi
    call    intern
    lea     sym_DIFFERENCE(%rip), %rdi
    call    intern
    lea     sym_PLUS(%rip), %rdi
    call    intern

    movq    $NIL_SYM, global_env(%rip)

    /* Auto-load startup.lisp, silently, if it exists in the current
     * directory -- before anything else, so its DEFINEs are already
     * in global_env for both file mode and the REPL. fopen returning
     * NULL (no such file) is not an error here, just "no startup
     * library today" -- skip cleanly, same as any other run. */
    lea     fname_startup(%rip), %rdi
    lea     fmt_mode_r(%rip), %rsi
    call    fopen
    test    %rax, %rax
    jz      .no_startup
    mov     %rax, %r12
    lea     filebuf(%rip), %rdi
    mov     $1, %rsi
    mov     $65535, %rdx
    mov     %r12, %rcx
    call    fread
    lea     filebuf(%rip), %rdi
    movb    $0, (%rdi,%rax,1)
    mov     %r12, %rdi
    call    fclose
    lea     filebuf(%rip), %rax
    mov     %rax, input_ptr(%rip)
    call    process_buffer
.no_startup:

    /* argc < 2 (no file argument) -> interactive REPL on stdin.
     * argc >= 2 -> load argv[1] and process it in one pass, as before. */
    cmpq    $2, argc_saved(%rip)
    jl      .repl_mode

    /* Load the program from the file named in argv[1]. */
    mov     8(%rbx), %rdi        /* argv[1] */
    lea     fmt_mode_r(%rip), %rsi
    call    fopen
    mov     %rax, %r12           /* FILE* */
    lea     filebuf(%rip), %rdi
    mov     $1, %rsi
    mov     $65535, %rdx
    mov     %r12, %rcx
    call    fread                /* rax = bytes actually read */
    lea     filebuf(%rip), %rdi
    movb    $0, (%rdi,%rax,1)    /* null-terminate the loaded text */
    mov     %r12, %rdi
    call    fclose

    lea     filebuf(%rip), %rax
    mov     %rax, input_ptr(%rip)
    call    process_buffer
    jmp     .main_done

.repl_mode:
    lea     fmt_prompt(%rip), %rdi
    xor     %eax, %eax
    call    printf
    xor     %edi, %edi           /* fflush(NULL) -- flush all open streams, so the
                                     prompt appears before fgets blocks for input */
    call    fflush

    lea     linebuf(%rip), %rdi
    mov     $4096, %rsi
    mov     stdin(%rip), %rdx
    call    fgets
    test    %rax, %rax
    jz      .repl_eof            /* NULL = EOF (Ctrl-D) */

    lea     linebuf(%rip), %rax
    mov     %rax, input_ptr(%rip)
    call    process_buffer
    jmp     .repl_mode

.repl_eof:
    lea     fmt_newline(%rip), %rdi
    xor     %eax, %eax
    call    printf

.main_done:
    pop     %rbx
    xor     %eax, %eax
    ret

/* process_buffer() -- reads and evaluates every top-level form from
 * the current input_ptr until a NUL byte, printing each result
 * (DEFINE prints nothing). Shared by both file mode and REPL mode,
 * so the two don't silently diverge in behavior. */
process_buffer:
    push    %rbx
.pb_loop:
    call    skip_ws
    mov     input_ptr(%rip), %rax
    movzx   (%rax), %eax
    test    %al, %al
    jz      .pb_done

    call    read_sexpr
    mov     %rax, %rbx           /* the top-level form just read */

    /* is it (DEFINE name expr)? -- DEFINE is top-level-only. */
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jnz     .pb_eval_plain
    mov     %rbx, %rdi
    call    car
    cmp     $DEFINE_SYM, %rax
    jne     .pb_eval_plain

    /* DEFINE binds name to the RAW, unevaluated form -- not eval[expr].
     * eval only recognizes LABEL/LAMBDA as the head of a call, never
     * as a standalone value-producing expression; plain_call's assoc
     * needs to get the raw (LABEL name (LAMBDA ...)) form back so it
     * can reconstruct and re-eval the call correctly. This makes
     * DEFINE a function-definition form (like defun), not a general
     * evaluated-constant binding -- a scoping choice, not an oversight. */
    mov     %rbx, %rdi
    call    cadr                 /* name */
    push    %rax
    mov     %rbx, %rdi
    call    caddr                /* raw definition form, e.g. (LABEL ...) */
    pop     %rdi                 /* name */
    mov     %rax, %rsi
    call    cons                 /* (name . raw-form) */
    mov     %rax, %rdi
    mov     global_env(%rip), %rsi
    call    cons                 /* ((name.value) . global_env) */
    mov     %rax, global_env(%rip)
    jmp     .pb_loop            /* DEFINE prints nothing; silent success */

.pb_eval_plain:
    mov     %rbx, %rdi
    mov     global_env(%rip), %rsi
    call    eval
    mov     %rax, %rdi
    call    print_sexpr
    lea     fmt_newline(%rip), %rdi
    xor     %eax, %eax
    call    printf
    jmp     .pb_loop

.pb_done:
    pop     %rbx
    ret

    .section .rodata
sym_QUOTE:  .asciz "QUOTE"
sym_ATOM:   .asciz "ATOM"
sym_EQ:     .asciz "EQ"
sym_COND:   .asciz "COND"
sym_CAR:    .asciz "CAR"
sym_CDR:    .asciz "CDR"
sym_CONS:   .asciz "CONS"
sym_LABEL:  .asciz "LABEL"
sym_LAMBDA: .asciz "LAMBDA"
sym_DEFINE: .asciz "DEFINE"
sym_ZEROP:      .asciz "ZEROP"
sym_TIMES:      .asciz "TIMES"
sym_DIFFERENCE: .asciz "DIFFERENCE"
sym_PLUS:       .asciz "PLUS"

    .section .note.GNU-stack,"",@progbits
