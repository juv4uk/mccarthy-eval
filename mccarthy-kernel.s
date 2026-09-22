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
fmt_condition_unbound:
    .asciz "CONDITION kind=UNBOUND name=%s\n"
fmt_condition_setq_unbound:
    .asciz "CONDITION kind=SETQ-UNBOUND name=%s\n"
fmt_condition_go_notfound:
    .asciz "CONDITION kind=GO-LABEL-NOT-FOUND name=%s\n"

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

proplist_table:             /* one property-list head per symbol, index-parallel
                              * to symtab (Appendix B ст.58-59, GET/DEFLIST/
                              * REMPROP) -- each slot holds a flat, alternating
                              * (indicator value indicator value ...) list, per
                              * the manual's own NIL/FF property-list diagrams.
                              * Initialized to NIL_SYM per-symbol in intern, not
                              * here -- NIL_SYM is 1, not 0, so a zeroed .skip
                              * region would NOT already mean "empty". */
    .skip 8 * 65536

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

/* equal_prim(rdi=x, rsi=y) -> rax (1/0). LISP 1.5 Manual (1962),
 * Appendix A: "equal is true if its arguments are the same
 * S-expression... It uses eq on the atomic level and is recursive."
 * Recursive, callee-saves r12/r13 like evlis/pair/append above. */
equal_prim:
    push    %r12
    push    %r13
    mov     %rdi, %r12
    mov     %rsi, %r13
    mov     %r12, %rdi
    call    atomp
    push    %rax
    mov     %r13, %rdi
    call    atomp
    mov     %rax, %rcx
    pop     %rax
    cmp     %rax, %rcx
    jne     .equal_prim_false   /* one is atomic, the other isn't */
    test    %rax, %rax
    jz      .equal_prim_recurse
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    eq_prim
    jmp     .equal_prim_done
.equal_prim_recurse:
    mov     %r12, %rdi
    call    car
    push    %rax
    mov     %r13, %rdi
    call    car
    mov     %rax, %rsi
    pop     %rdi
    call    equal_prim
    test    %rax, %rax
    jz      .equal_prim_false
    mov     %r12, %rdi
    call    cdr
    push    %rax
    mov     %r13, %rdi
    call    cdr
    mov     %rax, %rsi
    pop     %rdi
    call    equal_prim
    jmp     .equal_prim_done
.equal_prim_false:
    xor     %eax, %eax
.equal_prim_done:
    pop     %r13
    pop     %r12
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

cddr:
    call    cdr
    mov     %rax, %rdi
    jmp     cdr

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

/* prog_feature/prog_cond/prog_find_label/prog_setvar -- the PROG
 * feature (Appendix B ст.71 + the canonical worked LENGTH/REV
 * examples in the main body, "V. THE PROGRAM FEATURE", ст.29-30,
 * both verified by direct page-image read). PROG's statement list
 * is not evaluated by plain recursive descent like every other form
 * in this kernel -- it needs a genuine sequential walk with jumps,
 * so this is a real loop over a "current statement" pointer, not a
 * fold. GO re-scans the body from the start on every jump rather
 * than using the real system's own internal go-list cache -- a
 * reconstruction-derived equivalent (same precedent already used for
 * PAIR/REVERSE, rewritten from the source's own PROG/GO form into
 * direct recursion): identical observable behaviour, simpler control
 * flow, no new data structure. */
prog_feature:                     /* rdi = cdr[e] = (vars stmt...), rsi = a */
    push    %r12
    push    %r13
    push    %r14
    push    %r15
    push    %rbx
    mov     %rdi, %r12
    mov     %rsi, %r14            /* r14 = new_env, starts as a */
    mov     %r12, %rdi
    call    car
    mov     %rax, %r13            /* r13 = vars (walking, setup only) */
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rbx            /* rbx = body (FIXED, for GO rescans) */
    mov     %rax, %r15            /* r15 = cur (current statement ptr) */

.prog_bind_vars:
    cmp     $NIL_SYM, %r13
    je      .prog_bind_done
    mov     %r13, %rdi
    call    car
    mov     %rax, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                  /* (v . NIL) -- program vars start at NIL */
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    cons                  /* ((v . NIL) . new_env) */
    mov     %rax, %r14
    mov     %r13, %rdi
    call    cdr
    mov     %rax, %r13
    jmp     .prog_bind_vars
.prog_bind_done:

.prog_main_loop:
    cmp     $NIL_SYM, %r15
    je      .prog_return_nil      /* ran out of statements -> NIL */
    mov     %r15, %rdi
    call    car
    mov     %rax, %r12            /* r12 = stmt (prog_cdr no longer needed) */
    mov     %r12, %rdi
    call    atomp
    test    %rax, %rax
    jz      .prog_stmt_compound
    /* atomic top-level entry = a label -- inert when reached in
     * sequence, just skip past it */
    mov     %r15, %rdi
    call    cdr
    mov     %rax, %r15
    jmp     .prog_main_loop

.prog_stmt_compound:
    mov     %r12, %rdi
    call    car
    mov     %rax, %r13            /* r13 = head (stable across calls below) */

    cmp     $RETURN_SYM, %r13
    jne     .prog_try_go
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    eval
    jmp     .prog_done            /* rax already holds the return value */

.prog_try_go:
    cmp     $GO_SYM, %r13
    jne     .prog_try_setq
    mov     %r12, %rdi
    call    cadr                  /* target label, raw/unevaluated */
    mov     %rax, %rdi
    mov     %rbx, %rsi
    call    prog_find_label
    mov     %rax, %r15
    jmp     .prog_main_loop

.prog_try_setq:
    cmp     $SETQ_SYM, %r13
    jne     .prog_try_set
    mov     %r12, %rdi
    call    cadr                  /* var, raw/unevaluated (SETQ quotes it) */
    push    %rax
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    eval
    mov     %rax, %rsi
    pop     %rdi
    mov     %r14, %rdx
    call    prog_setvar
    mov     %r15, %rdi
    call    cdr
    mov     %rax, %r15
    jmp     .prog_main_loop

.prog_try_set:
    cmp     $SET_SYM, %r13
    jne     .prog_try_cond
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    eval                  /* SET evaluates its first arg too */
    push    %rax
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    eval
    mov     %rax, %rsi
    pop     %rdi
    mov     %r14, %rdx
    call    prog_setvar
    mov     %r15, %rdi
    call    cdr
    mov     %rax, %r15
    jmp     .prog_main_loop

.prog_try_cond:
    cmp     $COND_SYM, %r13
    jne     .prog_ordinary_stmt
    /* COND at the top level of a PROG has two peculiarities (ст.71):
     * (a) GO may appear as a clause's value part, jumping instead of
     * evaluating; (b) running out of clauses is NOT an error here --
     * the PROG just continues with its next statement. */
    mov     %r15, %rdi
    call    cdr                   /* fallthrough_cur = cdr[cur] */
    push    %rax
    mov     %r12, %rdi
    call    cdr                   /* clauses = cdr[stmt] */
    mov     %rax, %rdi
    mov     %r14, %rsi
    mov     %rbx, %rdx
    pop     %rcx
    call    prog_cond             /* rax=0 -> keep looping, cur in rdx;
                                    * rax=1 -> the whole PROG returns,
                                    * value in rdx (RETURN inside a
                                    * top-level COND clause -- the
                                    * manual's own LENGTH example) */
    test    %rax, %rax
    jnz     .prog_cond_returned
    mov     %rdx, %r15
    jmp     .prog_main_loop
.prog_cond_returned:
    mov     %rdx, %rax
    jmp     .prog_done

.prog_ordinary_stmt:
    /* "Executing a statement means evaluating it with the current
     * a-list and ignoring its value" (ст.30) */
    mov     %r12, %rdi
    mov     %r14, %rsi
    call    eval
    mov     %r15, %rdi
    call    cdr
    mov     %rax, %r15
    jmp     .prog_main_loop

.prog_return_nil:
    mov     $NIL_SYM, %rax

.prog_done:
    pop     %rbx
    pop     %r15
    pop     %r14
    pop     %r13
    pop     %r12
    ret

prog_cond:                        /* rdi=clauses, rsi=new_env, rdx=body, rcx=fallthrough_cur
                                    * returns: rax=0/rdx=next-cur, or
                                    * rax=1/rdx=prog's-final-value */
    push    %r12
    push    %r13
    push    %r14
    push    %r15
    push    %rbx
    mov     %rdi, %r12
    mov     %rsi, %r13
    mov     %rdx, %r14
    mov     %rcx, %r15
.prog_cond_loop:
    cmp     $NIL_SYM, %r12
    je      .prog_cond_fallthrough
    mov     %r12, %rdi
    call    car
    mov     %rax, %rbx            /* rbx = clause */
    mov     %rbx, %rdi
    call    car                   /* pred = car[clause] */
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    je      .prog_cond_next
    mov     %rbx, %rdi
    call    cadr                  /* valpart = cadr[clause] */
    mov     %rax, %rbx            /* rbx = valpart now */
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jnz     .prog_cond_eval_valpart
    mov     %rbx, %rdi
    call    car
    mov     %rax, %r8             /* head of valpart (scratch, no calls before use) */
    cmp     $GO_SYM, %r8
    je      .prog_cond_do_go
    cmp     $RETURN_SYM, %r8
    je      .prog_cond_do_return
    jmp     .prog_cond_eval_valpart
.prog_cond_do_go:
    /* the ONLY documented placement for GO besides a PROG's own top
     * level (ст.71, rule 5b) */
    mov     %rbx, %rdi
    call    cadr                  /* target label */
    mov     %rax, %rdi
    mov     %r14, %rsi
    call    prog_find_label
    mov     %rax, %rdx
    xor     %eax, %eax
    jmp     .prog_cond_done
.prog_cond_do_return:
    /* not in the appendix's own placement rule for GO, but the
     * manual's own canonical LENGTH example (ст.29-30) uses exactly
     * this -- (COND ((NULL U) (RETURN V))) -- so a top-level PROG
     * COND must also recognize RETURN in its value part, ending the
     * whole PROG, not merely evaluating (RETURN V) as an ordinary,
     * unrecognized call. */
    mov     %rbx, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdx
    mov     $1, %eax
    jmp     .prog_cond_done
.prog_cond_eval_valpart:
    mov     %rbx, %rdi
    mov     %r13, %rsi
    call    eval                  /* discard value -- it's a statement */
    mov     %r15, %rdx
    xor     %eax, %eax
    jmp     .prog_cond_done
.prog_cond_next:
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %r12
    jmp     .prog_cond_loop
.prog_cond_fallthrough:
    mov     %r15, %rdx
    xor     %eax, %eax
.prog_cond_done:
    pop     %rbx
    pop     %r15
    pop     %r14
    pop     %r13
    pop     %r12
    ret

prog_find_label:                  /* rdi=target, rsi=body */
    push    %r12
    push    %rbx
    mov     %rdi, %rbx             /* rbx = target */
    mov     %rsi, %r12             /* r12 = walking list pointer */
.prog_find_label_loop:
    cmp     $NIL_SYM, %r12
    je      .prog_find_label_notfound
    mov     %r12, %rdi
    call    car
    push    %rax                   /* save item across the atomp call */
    mov     %rax, %rdi
    call    atomp
    pop     %rdi                   /* item back into rdi */
    test    %rax, %rax
    jz      .prog_find_label_advance
    cmp     %rbx, %rdi
    jne     .prog_find_label_advance
    mov     %r12, %rdi
    call    cdr                    /* found -- return the remainder */
    jmp     .prog_find_label_done
.prog_find_label_advance:
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %r12
    jmp     .prog_find_label_loop
.prog_find_label_notfound:
    /* Real error A6 in the historical system ("GO refers to a
     * nonexistent location"). Graceful here, not a crash: report on
     * stderr and fall off the end (PROG returns NIL), matching every
     * other uncallable/unbound case in this kernel. */
    mov     %rbx, %rdx
    shr     $2, %rdx
    mov     symtab(,%rdx,8), %rdx
    mov     stderr(%rip), %rdi
    lea     fmt_condition_go_notfound(%rip), %rsi
    xor     %eax, %eax
    call    fprintf
    mov     $NIL_SYM, %rax
.prog_find_label_done:
    pop     %rbx
    pop     %r12
    ret

prog_setvar:                      /* rdi=var, rsi=val, rdx=env */
.prog_setvar_loop:
    cmp     $NIL_SYM, %rdx
    je      .prog_setvar_notfound
    mov     (%rdx), %rcx           /* pair = car[env-node] */
    mov     (%rcx), %r8            /* key = car[pair] */
    cmp     %r8, %rdi
    je      .prog_setvar_found
    mov     8(%rdx), %rdx          /* advance to cdr[env-node] */
    jmp     .prog_setvar_loop
.prog_setvar_found:
    mov     %rsi, 8(%rcx)          /* mutate cdr[pair] in place */
    ret
.prog_setvar_notfound:
    /* Real error A4/A5 in the historical system ("SETQ given on
     * nonexistent program variable"). Graceful here: report and
     * silently no-op, same posture as prog_find_label above. */
    mov     %rdi, %rdx
    shr     $2, %rdx
    mov     symtab(,%rdx,8), %rdx
    mov     stderr(%rip), %rdi
    lea     fmt_condition_setq_unbound(%rip), %rsi
    xor     %eax, %eax
    call    fprintf
    ret

/* get_prim/deflist_prim/remprop_prim -- property lists (Appendix B
 * ст.58-59, both verified by direct page-image read). Each symbol
 * gets its own separate property list, a flat, alternating
 * (indicator value indicator value ...) list, per the manual's own
 * NIL/FF property-list diagrams -- stored in proplist_table,
 * index-parallel to symtab. This kernel does not model the "-1
 * sentinel head" or the PNAME/EXPR/SUBR indicators the real system
 * uses internally for ITS OWN function/name storage (that's
 * 704/CTSS-specific representation detail this kernel doesn't share
 * -- functions here live in global_env via DEFINE, not on property
 * lists) -- only the general, user-visible GET/DEFLIST/REMPROP
 * facility itself.
 *
 * Reconstruction-derived correction, not a blind transcription: the
 * scanned image's own get[x;y] recursive step reads as
 * "get[cdr[x];y]" -- but a single cdr cannot be correct for a flat
 * alternating list (the very next car would be a VALUE, not an
 * indicator, on the following call). Implemented as get[cddr[x];y]
 * instead, the only structurally consistent reading, cross-checked
 * against the manual's own flat-list property diagrams and its
 * neighbouring prop[x;y;u] formula (which correctly uses a single
 * cdr for ITS OWN, different, one-step-at-a-time list scan). */
get_prim:                          /* rdi=symbol, rsi=indicator */
    mov     %rdi, %rdx
    shr     $2, %rdx
    mov     proplist_table(,%rdx,8), %rdi
    jmp     get_prim_walk

get_prim_walk:                     /* rdi=plist, rsi=indicator */
    push    %r12
    push    %r13
    mov     %rdi, %r12
    mov     %rsi, %r13
.get_prim_walk_loop:
    cmp     $NIL_SYM, %r12
    je      .get_prim_walk_nil
    mov     %r12, %rdi
    call    car
    cmp     %r13, %rax
    je      .get_prim_walk_found
    mov     %r12, %rdi
    call    cddr
    mov     %rax, %r12
    jmp     .get_prim_walk_loop
.get_prim_walk_found:
    mov     %r12, %rdi
    call    cadr
    jmp     .get_prim_walk_done
.get_prim_walk_nil:
    mov     $NIL_SYM, %rax
.get_prim_walk_done:
    pop     %r13
    pop     %r12
    ret

/* deflist[x;ind] -- x is a list of (u v) pairs; for each, prepends
 * ind and v onto u's own property list ("puts things on at the
 * front", ст.58 -- so a repeated indicator is shadowed by the newer
 * entry, never physically replaced, matching "the old value will be
 * replaced by the new one" as an observable effect of GET always
 * finding the front-most match first). Returns the list of u's,
 * consistent with define[x] = deflist[x;EXPR] and "the value of
 * define is the list of u's" (ст.58) -- deflist must return the same
 * shape for that equation to hold. */
deflist_prim:                      /* rdi=pairlist, rsi=ind */
    push    %r12
    push    %r13
    push    %r14
    push    %rbx
    mov     %rdi, %r12
    mov     %rsi, %r13
    cmp     $NIL_SYM, %r12
    je      .deflist_base
    mov     %r12, %rdi
    call    car
    mov     %rax, %r14              /* onepair = (u v) */
    mov     %r14, %rdi
    call    car
    mov     %rax, %rbx              /* rbx = u */
    mov     %r14, %rdi
    call    cadr                    /* v */
    mov     %rax, %rdi
    mov     %rbx, %rdx
    shr     $2, %rdx
    mov     proplist_table(,%rdx,8), %rsi
    call    cons                    /* (v . plist) */
    mov     %rax, %rsi
    mov     %r13, %rdi
    call    cons                    /* (ind v . plist) */
    mov     %rbx, %rdx
    shr     $2, %rdx
    mov     %rax, proplist_table(,%rdx,8)
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    deflist_prim            /* recurse -- filtered rest of the list of u's */
    mov     %rax, %rsi
    mov     %rbx, %rdi
    call    cons                    /* u . (rest of the u's) */
    jmp     .deflist_done
.deflist_base:
    mov     $NIL_SYM, %rax
.deflist_done:
    pop     %rbx
    pop     %r14
    pop     %r13
    pop     %r12
    ret

/* remprop[x;ind] -- x is a SYMBOL here (matching get/deflist's own
 * x=symbol convention, ст.58); removes ALL occurrences of ind (and
 * its following value) from x's property list. Value is always NIL
 * (ст.59). Rebuilds a filtered list rather than literally splicing
 * with RPLACD as the real system does -- same observable result,
 * simpler and safer than in-place list surgery in hand-written
 * assembly; not exposed to any caller as a semantic difference. */
remprop_prim:                      /* rdi=symbol, rsi=indicator */
    push    %r12
    push    %r13
    push    %r14
    mov     %rdi, %r14
    mov     %rsi, %r13
    mov     %r14, %rdx
    shr     $2, %rdx
    mov     proplist_table(,%rdx,8), %r12
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    remprop_filter
    mov     %r14, %rdx
    shr     $2, %rdx
    mov     %rax, proplist_table(,%rdx,8)
    mov     $NIL_SYM, %rax
    pop     %r14
    pop     %r13
    pop     %r12
    ret

remprop_filter:                    /* rdi=plist, rsi=indicator -> filtered plist */
    push    %r12
    push    %r13
    mov     %rdi, %r12
    mov     %rsi, %r13
    cmp     $NIL_SYM, %r12
    je      .remprop_filter_base
    mov     %r12, %rdi
    call    car
    cmp     %r13, %rax
    je      .remprop_filter_skip
    mov     %r12, %rdi
    call    car
    push    %rax                    /* this-ind */
    mov     %r12, %rdi
    call    cadr
    push    %rax                    /* this-val */
    mov     %r12, %rdi
    call    cddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    remprop_filter
    mov     %rax, %rsi
    pop     %rdi                     /* this-val */
    call    cons
    mov     %rax, %rsi
    pop     %rdi                     /* this-ind */
    call    cons
    jmp     .remprop_filter_done
.remprop_filter_skip:
    mov     %r12, %rdi
    call    cddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    remprop_filter
    jmp     .remprop_filter_done
.remprop_filter_base:
    mov     $NIL_SYM, %rax
.remprop_filter_done:
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
    jne     .try_function
    mov     %r12, %rdi
    call    cadr
    jmp     .eval_done

.try_function:
    /* eq[car[form];FUNCTION] -> list[FUNARG;cadr[form];a]
     * (Appendix B, LISP 1.5 Manual 1962, ст.71 -- real, source-
     * confirmed formula, verified by direct page-image read, not
     * OCR). FUNCTION wraps its argument together with the CURRENT
     * environment a into a 3-list (FUNARG fn a) -- this triple IS
     * this kernel's "function value" representation. Its own
     * argument, cadr[form], is used unevaluated on purpose, matching
     * the real formula exactly (fn is typically a raw (LAMBDA ...)
     * or (LABEL name (LAMBDA ...)) expression, not a value). */
    cmp     $FUNCTION_SYM, %r14
    jne     .try_prog
    mov     %r12, %rdi
    call    cadr
    push    %rax
    mov     %r13, %rdi
    mov     $NIL_SYM, %rsi
    call    cons
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    mov     %rax, %rsi
    mov     $FUNARG_SYM, %rdi
    call    cons
    jmp     .eval_done

.try_prog:
    /* eq[car[form];PROG] -> prog[cdr[form];a] (Appendix B, ст.71,
     * and the canonical worked example in the main body, "V. THE
     * PROGRAM FEATURE", ст.29-30 -- both verified by direct
     * page-image read). PROG/GO/RETURN/SETQ/SET are all handled
     * inside prog_feature itself, not as general eval special forms
     * -- the manual is explicit these are "peculiar to prog", never
     * meaningful outside one. */
    cmp     $PROG_SYM, %r14
    jne     .try_atom
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    prog_feature
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
    jne     .try_null
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

/* --- LISP 1.5 Appendix A primitives (issue #7 Phase 2), "стосується
 * заліза" -- real x86-64 instructions, not LABEL/LAMBDA library code.
 * Exact page citations: tests/historical-facility-extensions/PROVENANCE.md */

.try_null:
    cmp     $NULL_SYM, %r14
    jne     .try_equal
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    je      .null_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.null_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

.try_equal:
    cmp     $EQUAL_SYM, %r14
    jne     .try_list
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
    call    equal_prim
    test    %rax, %rax
    jz      .equal_call_false
    mov     $T_SYM, %rax
    jmp     .eval_done
.equal_call_false:
    mov     $NIL_SYM, %rax
    jmp     .eval_done

/* list[x1;...;xn] -- evlis already builds exactly this list. */
.try_list:
    cmp     $LIST_SYM, %r14
    jne     .try_and
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    jmp     .eval_done

/* and[x1;...;xn] -- FSUBR, short-circuit (not evlis: must stop at the
 * first false without evaluating the rest). Value is T or NIL, per
 * the Manual's own wording ("the value of and is false or true
 * respectively"), not the last evaluated value. */
.try_and:
    cmp     $AND_SYM, %r14
    jne     .try_or
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rbx
.and_loop:
    cmp     $NIL_SYM, %rbx
    je      .and_true
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    je      .and_false
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .and_loop
.and_true:
    mov     $T_SYM, %rax
    jmp     .and_done
.and_false:
    mov     $NIL_SYM, %rax
.and_done:
    pop     %rbx
    jmp     .eval_done

.try_or:
    cmp     $OR_SYM, %r14
    jne     .try_not
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rbx
.or_loop:
    cmp     $NIL_SYM, %rbx
    je      .or_false
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    jne     .or_true
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .or_loop
.or_true:
    mov     $T_SYM, %rax
    jmp     .or_done
.or_false:
    mov     $NIL_SYM, %rax
.or_done:
    pop     %rbx
    jmp     .eval_done

.try_not:
    cmp     $NOT_SYM, %r14
    jne     .try_rplaca
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    cmp     $NIL_SYM, %rax
    je      .not_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.not_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

/* rplaca[x;y] / rplacd[x;y] -- pseudo-functions, destructive. Guarded
 * the same way car/cdr already guard: a non-pointer (bit0==1) is left
 * untouched rather than dereferenced, same fallback philosophy as the
 * rest of this kernel. Value is x (the modified pair), conventional. */
.try_rplaca:
    cmp     $RPLACA_SYM, %r14
    jne     .try_rplacd
    push    %rbx
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rbx
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rsi
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jnz     .rplaca_done
    mov     %rsi, (%rbx)
.rplaca_done:
    mov     %rbx, %rax
    pop     %rbx
    jmp     .eval_done

.try_rplacd:
    cmp     $RPLACD_SYM, %r14
    jne     .try_get
    push    %rbx
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rbx
    mov     %r12, %rdi
    call    caddr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rsi
    mov     %rbx, %rdi
    call    atomp
    test    %rax, %rax
    jnz     .rplacd_done
    mov     %rsi, 8(%rbx)
.rplacd_done:
    mov     %rbx, %rax
    pop     %rbx
    jmp     .eval_done

.try_get:
    /* get[x;y] (Appendix B ст.58-59) -- ordinary primitive, both
     * args evaluated. x must evaluate to a symbol. */
    cmp     $GET_SYM, %r14
    jne     .try_deflist
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
    call    get_prim
    jmp     .eval_done

.try_deflist:
    cmp     $DEFLIST_SYM, %r14
    jne     .try_remprop
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
    call    deflist_prim
    jmp     .eval_done

.try_remprop:
    cmp     $REMPROP_SYM, %r14
    jne     .try_minus
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
    call    remprop_prim
    jmp     .eval_done

.try_minus:
    cmp     $MINUS_SYM, %r14
    jne     .try_add1
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    neg     %rax
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.try_add1:
    cmp     $ADD1_SYM, %r14
    jne     .try_sub1
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    inc     %rax
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.try_sub1:
    cmp     $SUB1_SYM, %r14
    jne     .try_max
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    dec     %rax
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

/* max/min[x1;...;xn] -- no neutral identity element is defined for
 * these (unlike plus/times), so the first evaluated argument seeds
 * the fold; an empty argument list is undefined by the Manual and
 * degrades harmlessly here rather than crashing. */
.try_max:
    cmp     $MAX_SYM, %r14
    jne     .try_min
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    push    %rax
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
.max_loop:
    cmp     $NIL_SYM, %rbx
    je      .max_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    cmp     %rcx, %rax
    cmovg   %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .max_loop
.max_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

.try_min:
    cmp     $MIN_SYM, %r14
    jne     .try_recip
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    push    %rax
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
.min_loop:
    cmp     $NIL_SYM, %rbx
    je      .min_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    cmp     %rcx, %rax
    cmovl   %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .min_loop
.min_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

/* recip[x] = quotient[1;x] -- exactly the Manual's own formula; for a
 * fixnum-only kernel this already gives 0 for |x|>1 by construction
 * ("the reciprocal of any fixed point number is defined as zero"). */
.try_recip:
    cmp     $RECIP_SYM, %r14
    jne     .try_quotient
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    mov     %rax, %rcx
    mov     $1, %rax
    cqto
    idiv    %rcx
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.try_quotient:
    cmp     $QUOTIENT_SYM, %r14
    jne     .try_remainder
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
    mov     %rax, %rcx
    pop     %rax
    cqto
    idiv    %rcx
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.try_remainder:
    cmp     $REMAINDER_SYM, %r14
    jne     .try_divide
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
    mov     %rax, %rcx
    pop     %rax
    cqto
    idiv    %rcx
    mov     %rdx, %rdi
    call    mkfix
    jmp     .eval_done

/* divide[x;y] = cons[quotient[x;y];remainder[x;y]] -- exact Manual
 * formula, one idiv gives both halves directly. */
.try_divide:
    cmp     $DIVIDE_SYM, %r14
    jne     .try_expt
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
    mov     %rax, %rcx
    pop     %rax
    cqto
    idiv    %rcx
    push    %rdx
    mov     %rax, %rdi
    call    mkfix
    mov     %rax, %rdi
    pop     %rax
    push    %rdi
    mov     %rax, %rdi
    call    mkfix
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    jmp     .eval_done

/* expt[x;y] = x^y, y>=0, iterative multiplication (fixed-point only,
 * exactly as the Manual specifies for this case). */
.try_expt:
    cmp     $EXPT_SYM, %r14
    jne     .try_lessp
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
    mov     %rax, %rcx
    pop     %rdx
    mov     $1, %rax
.expt_loop:
    test    %rcx, %rcx
    jz      .expt_done
    imul    %rdx, %rax
    dec     %rcx
    jmp     .expt_loop
.expt_done:
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.try_lessp:
    cmp     $LESSP_SYM, %r14
    jne     .try_greaterp
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
    cmp     %rax, %rcx
    jl      .lessp_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.lessp_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

.try_greaterp:
    cmp     $GREATERP_SYM, %r14
    jne     .try_onep
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
    cmp     %rax, %rcx
    jg      .greaterp_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.greaterp_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

.try_onep:
    cmp     $ONEP_SYM, %r14
    jne     .try_minusp
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    cmp     $1, %rax
    je      .onep_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.onep_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

.try_minusp:
    cmp     $MINUSP_SYM, %r14
    jne     .try_numberp
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    getfix
    test    %rax, %rax
    js      .minusp_true
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.minusp_true:
    mov     $T_SYM, %rax
    jmp     .eval_done

.try_numberp:
    cmp     $NUMBERP_SYM, %r14
    jne     .try_fixp
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    fixnump
    test    %rax, %rax
    jz      .numberp_false
    mov     $T_SYM, %rax
    jmp     .eval_done
.numberp_false:
    mov     $NIL_SYM, %rax
    jmp     .eval_done

/* fixp[x] -- this kernel is fixnum-only, so fixp and numberp coincide
 * exactly (there is no separate floating-point type to distinguish
 * them from, per floatp below). */
.try_fixp:
    cmp     $FIXP_SYM, %r14
    jne     .try_floatp
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     %rax, %rdi
    call    fixnump
    test    %rax, %rax
    jz      .fixp_false
    mov     $T_SYM, %rax
    jmp     .eval_done
.fixp_false:
    mov     $NIL_SYM, %rax
    jmp     .eval_done

/* floatp[x] -- always NIL: this kernel implements no floating-point
 * type at all (an honest, real boundary, not a silent omission; see
 * README.md/ZEROP's own fixed-point-only narrowing for the same
 * reasoning). The argument is still evaluated for consistency with
 * every other predicate here, even though the answer never depends
 * on it. */
.try_floatp:
    cmp     $FLOATP_SYM, %r14
    jne     .try_logor
    mov     %r12, %rdi
    call    cadr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    eval
    mov     $NIL_SYM, %rax
    jmp     .eval_done

.try_logor:
    cmp     $LOGOR_SYM, %r14
    jne     .try_logand
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     $0, %rax
    push    %rax
.logor_loop:
    cmp     $NIL_SYM, %rbx
    je      .logor_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    or      %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .logor_loop
.logor_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

.try_logand:
    cmp     $LOGAND_SYM, %r14
    jne     .try_logxor
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     $-1, %rax
    push    %rax
.logand_loop:
    cmp     $NIL_SYM, %rbx
    je      .logand_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    and     %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .logand_loop
.logand_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

.try_logxor:
    cmp     $LOGXOR_SYM, %r14
    jne     .try_leftshift
    push    %rbx
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis
    mov     %rax, %rbx
    mov     $0, %rax
    push    %rax
.logxor_loop:
    cmp     $NIL_SYM, %rbx
    je      .logxor_done
    mov     %rbx, %rdi
    call    car
    mov     %rax, %rdi
    call    getfix
    pop     %rcx
    xor     %rax, %rcx
    push    %rcx
    mov     %rbx, %rdi
    call    cdr
    mov     %rax, %rbx
    jmp     .logxor_loop
.logxor_done:
    pop     %rdi
    call    mkfix
    pop     %rbx
    jmp     .eval_done

/* leftshift[x;n] = x * 2^n; negative n shifts right (Manual's own
 * wording). Variable-count shl/sar via %cl, baseline x86-64. */
.try_leftshift:
    cmp     $LEFTSHIFT_SYM, %r14
    jne     .plain_call
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
    mov     %rax, %rcx
    pop     %rax
    test    %rcx, %rcx
    js      .leftshift_right
    shl     %cl, %rax
    jmp     .leftshift_done
.leftshift_right:
    neg     %rcx
    sar     %cl, %rax
.leftshift_done:
    mov     %rax, %rdi
    call    mkfix
    jmp     .eval_done

.plain_call:
    /* T -> eval[cons[assoc[car[e];a]; appq[evlis[cdr[e];a]]]; a]
     * (the appq[...] is the fix above -- 1960 paper's apply does
     * this too, cons[f;appq[args]], for exactly this reason)
     *
     * Real bug found and fixed 2026-09-22 (issue #28 follow-up, live
     * testing after ENV's removal in #21): assoc's not-found case
     * returns NIL_SYM -- the same sentinel this kernel already, and
     * deliberately, uses to make unbound atoms self-evaluate to NIL
     * (see the T/NIL self-evaluation comment in the atom branch
     * above). Calling an unbound symbol as a function used to build
     * cons[NIL_SYM;args] and re-eval it regardless; since NIL_SYM is
     * itself unbound too, that new expression's own head resolves
     * through this exact path again, producing the same expression
     * forever -- unbounded recursion, stack overflow, segfault, not
     * a clean result. (ENV) reproduced this directly once ENV.stopped
     * being a recognized top-level form, but it was never
     * ENV-specific: any typo'd or undefined function name in
     * head position triggered it. Fix: if assoc found nothing,
     * there is no function to call -- return NIL_SYM immediately,
     * consistent with how this kernel already treats every other
     * unbound-atom case, instead of trying to apply it. */
    mov     %r14, %rdi
    mov     %r13, %rsi
    call    assoc
    cmp     $NIL_SYM, %rax
    jne     .plain_call_found
    /* Diagnostic only, added 2026-09-22 -- a stderr side-channel, not
     * a semantic change. The VALUE returned to the running program is
     * still NIL, byte-for-byte the same as before this line existed;
     * this kernel keeps using the old Lisp's own value semantics
     * (unbound atom -> NIL) exactly as already documented above.
     * stdout (what every fixture's .expected compares against) is
     * untouched -- this writes to stderr only, so #9's own
     * 22-plain-call-unbound-function-no-crash.lisp still expects and
     * gets NIL on stdout. Format inspired by wsm-os-lisp's own REPL
     * condition-reporting convention (`CONDITION kind=... `), not
     * copied from any Rust source -- this file stays pure asm. */
    mov     %r14, %rdx
    shr     $2, %rdx
    mov     symtab(,%rdx,8), %rdx
    mov     stderr(%rip), %rdi
    lea     fmt_condition_unbound(%rip), %rsi
    xor     %eax, %eax
    call    fprintf
    mov     $NIL_SYM, %rax
    jmp     .eval_done
.plain_call_found:
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
    jne     .try_funarg
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
    jmp     .eval_done

.try_funarg:
    /* eq[car[fn];FUNARG] -> apply[cadr[fn];args;caddr[fn]] (Appendix
     * B, ст.70 -- car[e] here IS the FUNARG triple itself, produced
     * earlier by FUNCTION, sitting directly in call-head position;
     * this is the real historical closure mechanism, not invented
     * semantics). Arguments are evaluated under the CURRENT env
     * (r13, the call site) exactly as any other call -- only the
     * wrapped function's own body runs under the CAPTURED env
     * (caddr of the triple) instead of r13. Reuses the same
     * reconstruct-and-re-eval trick .plain_call already relies on:
     * build (innerfn . evaluated-args) and hand it to eval under the
     * captured environment, letting eval's own LABEL/LAMBDA dispatch
     * (above) do the actual application. */
    cmp     $FUNARG_SYM, %rax
    jne     .try_compute_head
    mov     %r12, %rdi
    call    car                 /* (FUNARG innerfn captured_env) */
    mov     %rax, %rdi
    call    caddr               /* captured_env */
    push    %rax
    mov     %r12, %rdi
    call    car
    mov     %rax, %rdi
    call    cadr                /* innerfn */
    push    %rax
    mov     %r12, %rdi
    call    cdr                 /* raw args */
    mov     %rax, %rdi
    mov     %r13, %rsi          /* evaluate args under CURRENT env */
    call    evlis
    mov     %rax, %rdi
    call    appq
    mov     %rax, %rsi
    pop     %rdi                /* innerfn */
    call    cons                /* (innerfn . appq'd-args) */
    mov     %rax, %rdi
    pop     %rsi                /* captured_env */
    call    eval
    jmp     .eval_done

.try_compute_head:
    /* T -> apply[eval[fn;a];args;a] (Appendix B, ст.70, apply's own
     * final fallback) -- car[e] is compound but headed by neither
     * LABEL, LAMBDA, nor FUNARG: it must be an arbitrary expression
     * that itself COMPUTES a function value at the call site, e.g.
     * ((MAKE-ADDER 5) 3) or ((FUNCTION FOO) 3). Evaluate car[e]
     * under the current env to find out what it actually is, then
     * feed the result back through eval's own generic dispatch --
     * the same reconstruct-and-re-eval mechanism used throughout
     * this kernel (.plain_call, .try_funarg above). If the computed
     * value is itself LABEL/LAMBDA/FUNARG-headed, this correctly
     * recurses into the matching branch above; anything else reports
     * CONDITION kind=UNBOUND the same way any other uncallable head
     * already does, once the reconstructed call's own head is
     * dispatched as a plain atom. */
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
    mov     %rax, %rdi
    call    appq
    mov     %rax, %rsi
    pop     %rdi
    call    cons
    mov     %rax, %rdi
    mov     %r13, %rsi
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
    movq    $NIL_SYM, proplist_table(,%r13,8)
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

/* peek_dot() -> rax=1 if a standalone "." token is next (surrounded
 * by whitespace/parens/EOF on both sides, matching the printer's own
 * dotted-pair output convention "(A . B)") -- and CONSUMES just the
 * "." character itself, leaving the rest of input untouched. rax=0
 * and input_ptr left alone otherwise. A "." embedded in a longer
 * token (there are none in this kernel -- no floats, per
 * looks_numeric's own comment above) would never reach here anyway,
 * since read_atom's tokenizer only stops at whitespace/parens/EOF. */
peek_dot:
    call    skip_ws
    mov     input_ptr(%rip), %rdi
    movzx   (%rdi), %eax
    cmp     $'.', %al
    jne     .peek_dot_no
    movzx   1(%rdi), %ecx
    test    %cl, %cl
    jz      .peek_dot_yes
    cmp     $' ', %cl
    je      .peek_dot_yes
    cmp     $'\t', %cl
    je      .peek_dot_yes
    cmp     $'\n', %cl
    je      .peek_dot_yes
    cmp     $'(', %cl
    je      .peek_dot_yes
    cmp     $')', %cl
    je      .peek_dot_yes
    jmp     .peek_dot_no
.peek_dot_yes:
    inc     %rdi
    mov     %rdi, input_ptr(%rip)
    mov     $1, %eax
    ret
.peek_dot_no:
    xor     %eax, %eax
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
    push    %rax                   /* car */
    call    peek_dot
    test    %rax, %rax
    jz      .read_list_proper
    /* dotted-pair tail: "(A . B)" -- read exactly one more sexpr as
     * the cdr, matching the printer's own dotted-pair notation
     * (previously only the printer could produce this syntax; the
     * reader could not parse it back -- tests/lisp15-library-
     * functions/PROVENANCE.md documented this asymmetry). */
    call    read_sexpr             /* cdr */
    push    %rax
    call    skip_ws
    mov     input_ptr(%rip), %rdi
    movzx   (%rdi), %eax
    cmp     $')', %al
    jne     .read_list_dot_close   /* malformed: no closing paren --
                                     * graceful, don't consume, don't
                                     * crash */
    inc     %rdi
    mov     %rdi, input_ptr(%rip)
.read_list_dot_close:
    pop     %rsi                   /* cdr */
    pop     %rdi                   /* car */
    call    cons
    ret
.read_list_proper:
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
/* LISP 1.5 Programmer's Manual (1962), Appendix A, "Functions and
 * Constants in the LISP System... as of August 1962" -- real,
 * physical x86-64 primitives (not LABEL/LAMBDA library functions;
 * those belong in a .lisp library file, not here). Exact page
 * citations in tests/historical-facility-extensions/PROVENANCE.md. */
.equ NULL_SYM,      65
.equ EQUAL_SYM,      69
.equ LIST_SYM,       73
.equ AND_SYM,        77
.equ OR_SYM,         81
.equ NOT_SYM,        85
.equ RPLACA_SYM,     89
.equ RPLACD_SYM,     93
.equ MINUS_SYM,      97
.equ ADD1_SYM,       101
.equ SUB1_SYM,       105
.equ MAX_SYM,        109
.equ MIN_SYM,        113
.equ RECIP_SYM,      117
.equ QUOTIENT_SYM,   121
.equ REMAINDER_SYM,  125
.equ DIVIDE_SYM,     129
.equ EXPT_SYM,       133
.equ LESSP_SYM,      137
.equ GREATERP_SYM,   141
.equ ONEP_SYM,       145
.equ MINUSP_SYM,     149
.equ NUMBERP_SYM,    153
.equ FIXP_SYM,       157
.equ FLOATP_SYM,     161
.equ LOGOR_SYM,      165
.equ LOGAND_SYM,     169
.equ LOGXOR_SYM,     173
.equ LEFTSHIFT_SYM,  177
.equ FUNCTION_SYM,   181
.equ FUNARG_SYM,     185
.equ PROG_SYM,       189
.equ GO_SYM,         193
.equ RETURN_SYM,     197
.equ SETQ_SYM,       201
.equ SET_SYM,        205
.equ GET_SYM,        209
.equ DEFLIST_SYM,    213
.equ REMPROP_SYM,    217

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
    lea     sym_NULL(%rip), %rdi
    call    intern
    lea     sym_EQUAL(%rip), %rdi
    call    intern
    lea     sym_LIST(%rip), %rdi
    call    intern
    lea     sym_AND(%rip), %rdi
    call    intern
    lea     sym_OR(%rip), %rdi
    call    intern
    lea     sym_NOT(%rip), %rdi
    call    intern
    lea     sym_RPLACA(%rip), %rdi
    call    intern
    lea     sym_RPLACD(%rip), %rdi
    call    intern
    lea     sym_MINUS(%rip), %rdi
    call    intern
    lea     sym_ADD1(%rip), %rdi
    call    intern
    lea     sym_SUB1(%rip), %rdi
    call    intern
    lea     sym_MAX(%rip), %rdi
    call    intern
    lea     sym_MIN(%rip), %rdi
    call    intern
    lea     sym_RECIP(%rip), %rdi
    call    intern
    lea     sym_QUOTIENT(%rip), %rdi
    call    intern
    lea     sym_REMAINDER(%rip), %rdi
    call    intern
    lea     sym_DIVIDE(%rip), %rdi
    call    intern
    lea     sym_EXPT(%rip), %rdi
    call    intern
    lea     sym_LESSP(%rip), %rdi
    call    intern
    lea     sym_GREATERP(%rip), %rdi
    call    intern
    lea     sym_ONEP(%rip), %rdi
    call    intern
    lea     sym_MINUSP(%rip), %rdi
    call    intern
    lea     sym_NUMBERP(%rip), %rdi
    call    intern
    lea     sym_FIXP(%rip), %rdi
    call    intern
    lea     sym_FLOATP(%rip), %rdi
    call    intern
    lea     sym_LOGOR(%rip), %rdi
    call    intern
    lea     sym_LOGAND(%rip), %rdi
    call    intern
    lea     sym_LOGXOR(%rip), %rdi
    call    intern
    lea     sym_LEFTSHIFT(%rip), %rdi
    call    intern
    lea     sym_FUNCTION(%rip), %rdi
    call    intern
    lea     sym_FUNARG(%rip), %rdi
    call    intern
    lea     sym_PROG(%rip), %rdi
    call    intern
    lea     sym_GO(%rip), %rdi
    call    intern
    lea     sym_RETURN(%rip), %rdi
    call    intern
    lea     sym_SETQ(%rip), %rdi
    call    intern
    lea     sym_SET(%rip), %rdi
    call    intern
    lea     sym_GET(%rip), %rdi
    call    intern
    lea     sym_DEFLIST(%rip), %rdi
    call    intern
    lea     sym_REMPROP(%rip), %rdi
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
sym_NULL:       .asciz "NULL"
sym_EQUAL:      .asciz "EQUAL"
sym_LIST:       .asciz "LIST"
sym_AND:        .asciz "AND"
sym_OR:         .asciz "OR"
sym_NOT:        .asciz "NOT"
sym_RPLACA:     .asciz "RPLACA"
sym_RPLACD:     .asciz "RPLACD"
sym_MINUS:      .asciz "MINUS"
sym_ADD1:       .asciz "ADD1"
sym_SUB1:       .asciz "SUB1"
sym_MAX:        .asciz "MAX"
sym_MIN:        .asciz "MIN"
sym_RECIP:      .asciz "RECIP"
sym_QUOTIENT:   .asciz "QUOTIENT"
sym_REMAINDER:  .asciz "REMAINDER"
sym_DIVIDE:     .asciz "DIVIDE"
sym_EXPT:       .asciz "EXPT"
sym_LESSP:      .asciz "LESSP"
sym_GREATERP:   .asciz "GREATERP"
sym_ONEP:       .asciz "ONEP"
sym_MINUSP:     .asciz "MINUSP"
sym_NUMBERP:    .asciz "NUMBERP"
sym_FIXP:       .asciz "FIXP"
sym_FLOATP:     .asciz "FLOATP"
sym_LOGOR:      .asciz "LOGOR"
sym_LOGAND:     .asciz "LOGAND"
sym_LOGXOR:     .asciz "LOGXOR"
sym_LEFTSHIFT:  .asciz "LEFTSHIFT"
sym_FUNCTION:   .asciz "FUNCTION"
sym_FUNARG:     .asciz "FUNARG"
sym_PROG:       .asciz "PROG"
sym_GO:         .asciz "GO"
sym_RETURN:     .asciz "RETURN"
sym_SETQ:       .asciz "SETQ"
sym_SET:        .asciz "SET"
sym_GET:        .asciz "GET"
sym_DEFLIST:    .asciz "DEFLIST"
sym_REMPROP:    .asciz "REMPROP"

    .section .note.GNU-stack,"",@progbits
