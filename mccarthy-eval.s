/* mccarthy-eval.s -- McCarthy 1960 eval/apply, hand-translated to real
 * x86_64 (System V AMD64 ABI), GNU assembler (AT&T syntax).
 *
 * Representation: 64-bit tagged word.
 *   bit0 == 1  -> immediate symbol/atom (payload = value >> 1)
 *   bit0 == 0  -> pointer to a 16-byte cons cell (car:8, cdr:8),
 *                 always 16-byte aligned so bit0 is naturally 0.
 *
 * Implements exactly the branches exercised by the two traces in
 * docs/correspondence/mccarthy-1960-eval-apply-walkthrough-2026-08-28.md:
 *   Trace 1: eval[(CONS (QUOTE A) (QUOTE B)); NIL]        = (A . B)
 *   Trace 2: eval[((LAMBDA (X) (CONS X X)) (QUOTE A)); NIL] = (A . A)
 * Branches present: atom->assoc, QUOTE, CAR, CDR, CONS, LAMBDA
 * (with real append/pair/evlis). LABEL, ATOM, EQ, COND are not
 * exercised by either trace and are omitted -- not because they're
 * hard, but because nothing here calls them; adding one is the same
 * pattern as CAR/CDR, not a new idea.
 */

    .section .rodata
fmt_result:
    .asciz "(%c . %c)\n"

    .section .bss
    .align 16
heap:
    .skip 16 * 4096          /* 4096 cons cells, bump-allocated */
heap_ptr:
    .skip 8

    .text
    .globl main

/* ---- symbol table (tagged immediates: (id << 1) | 1) ---- */
.equ NIL_SYM,    1     /* id 0 */
.equ A_SYM,      3     /* id 1 */
.equ B_SYM,      5     /* id 2 */
.equ QUOTE_SYM,  7     /* id 3 */
.equ CONS_SYM,   9     /* id 4 */
.equ CAR_SYM,    11    /* id 5 */
.equ CDR_SYM,    13    /* id 6 */
.equ LAMBDA_SYM, 15    /* id 7 */
.equ X_SYM,      17    /* id 8 */

/* ---------------------------------------------------------------
 * cons(rdi=car_val, rsi=cdr_val) -> rax = tagged pointer
 * --------------------------------------------------------------- */
cons:
    mov     heap_ptr(%rip), %rax
    mov     %rdi, (%rax)
    mov     %rsi, 8(%rax)
    lea     16(%rax), %rdx
    mov     %rdx, heap_ptr(%rip)
    ret

car:                            /* car(rdi=ptr) -> rax */
    mov     (%rdi), %rax
    ret

cdr:                            /* cdr(rdi=ptr) -> rax */
    mov     8(%rdi), %rax
    ret

atomp:                          /* atomp(rdi=val) -> rax (1/0) */
    mov     %rdi, %rax
    and     $1, %rax
    ret

eq_prim:                        /* eq_prim(rdi=a, rsi=b) -> rax (1/0) */
    xor     %eax, %eax
    cmp     %rsi, %rdi
    sete    %al
    ret

/* ---------------------------------------------------------------
 * cxr helpers -- pure composition of car/cdr
 * --------------------------------------------------------------- */
cadr:                           /* car(cdr(rdi)) */
    call    cdr
    mov     %rax, %rdi
    jmp     car

caddr:                          /* car(cdr(cdr(rdi))) */
    call    cdr
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

caar:                           /* car(car(rdi)) */
    call    car
    mov     %rax, %rdi
    jmp     car

cadar:                          /* car(cdr(car(rdi))) */
    call    car
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

caddar:                         /* car(cdr(cdr(car(rdi)))) */
    call    car
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    call    cdr
    mov     %rax, %rdi
    jmp     car

/* ---------------------------------------------------------------
 * assoc(rdi=sym, rsi=alist) -> rax  (tail-iterative)
 * --------------------------------------------------------------- */
assoc:
.assoc_loop:
    cmp     $NIL_SYM, %rsi
    je      .assoc_nil
    mov     (%rsi), %rdx        /* pair = car(alist) */
    mov     (%rdx), %rcx        /* key  = car(pair)  */
    cmp     %rcx, %rdi
    je      .assoc_found
    mov     8(%rsi), %rsi       /* alist = cdr(alist) */
    jmp     .assoc_loop
.assoc_found:
    mov     8(%rdx), %rax       /* cdr(pair) */
    ret
.assoc_nil:
    mov     $NIL_SYM, %rax
    ret

/* ---------------------------------------------------------------
 * pair(rdi=x, rsi=y) -> rax -- zip two lists into cons(x_i.y_i)'s
 * --------------------------------------------------------------- */
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
    mov     %rax, %rbx          /* rbx = car(x) */
    mov     %r13, %rdi
    call    car                 /* rax = car(y) */
    mov     %rbx, %rdi
    mov     %rax, %rsi
    call    cons                /* head = (car(x) . car(y)) */
    push    %rax
    mov     %r12, %rdi
    call    cdr
    mov     %rax, %r12
    mov     %r13, %rdi
    call    cdr
    mov     %rax, %r13
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    pair                /* tail = pair(cdr x, cdr y) */
    mov     %rax, %rsi
    pop     %rdi                /* head */
    call    cons
    jmp     .pair_done
.pair_nil:
    mov     $NIL_SYM, %rax
.pair_done:
    pop     %rbx
    pop     %r13
    pop     %r12
    ret

/* ---------------------------------------------------------------
 * append(rdi=l1, rsi=l2) -> rax
 * --------------------------------------------------------------- */
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

/* ---------------------------------------------------------------
 * evlis(rdi=m, rsi=a) -> rax
 * --------------------------------------------------------------- */
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

/* ---------------------------------------------------------------
 * eval(rdi=e, rsi=a) -> rax -- the main dispatcher
 * --------------------------------------------------------------- */
eval:
    push    %r12
    push    %r13
    push    %r14
    mov     %rdi, %r12          /* e */
    mov     %rsi, %r13          /* a */

    mov     %r12, %rdi
    call    atomp
    test    %rax, %rax
    jz      .not_atom
    mov     %r12, %rdi
    mov     %r13, %rsi
    call    assoc
    jmp     .eval_done

.not_atom:
    mov     %r12, %rdi
    call    car
    mov     %rax, %r14          /* head = car[e] */
    mov     %r14, %rdi
    call    atomp
    test    %rax, %rax
    jz      .head_not_atom

    cmp     $QUOTE_SYM, %r14
    jne     .try_car
    mov     %r12, %rdi
    call    cadr
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
    jne     .eval_done          /* T-> plain-call fallback: neither trace needs it */
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

.head_not_atom:
    mov     %r12, %rdi
    call    caar
    cmp     $LAMBDA_SYM, %rax
    jne     .eval_done          /* LABEL not exercised by either trace */

    mov     %r12, %rdi
    call    cdr                 /* raw argument expressions */
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    evlis               /* evaluated args */
    push    %rax
    mov     %r12, %rdi
    call    cadar               /* parameter list */
    mov     %rax, %rdi
    pop     %rsi
    call    pair                /* new bindings */
    mov     %rax, %rdi
    mov     %r13, %rsi
    call    append               /* new environment */
    push    %rax
    mov     %r12, %rdi
    call    caddar               /* lambda body */
    mov     %rax, %rdi
    pop     %rsi
    call    eval                 /* eval[body; new-env] */

.eval_done:
    pop     %r14
    pop     %r13
    pop     %r12
    ret

/* ---------------------------------------------------------------
 * print_sym(rdi=tagged symbol) -> al = ASCII char
 * --------------------------------------------------------------- */
print_sym:
    cmp     $A_SYM, %rdi
    je      .is_a
    cmp     $B_SYM, %rdi
    je      .is_b
    mov     $'?', %al
    ret
.is_a:
    mov     $'A', %al
    ret
.is_b:
    mov     $'B', %al
    ret

/* ---------------------------------------------------------------
 * print_pair(rdi=cons cell) -- prints "(car . cdr)\n" via printf
 * --------------------------------------------------------------- */
print_pair:
    push    %rbx                /* keeps 16-byte alignment for printf below */
    mov     %rdi, %rbx
    mov     (%rbx), %rdi
    call    print_sym
    movzx   %al, %edx            /* edx = car char, arg2 of "%c . %c" */
    push    %rdx
    mov     8(%rbx), %rdi
    call    print_sym
    movzx   %al, %ecx            /* ecx = cdr char, arg3 */
    pop     %rdx
    lea     fmt_result(%rip), %rdi
    mov     %edx, %esi
    mov     %ecx, %edx
    xor     %eax, %eax
    call    printf
    pop     %rbx
    ret

/* =================================================================
 * main -- build both traced expressions as real in-memory list
 * structure via cons, run them through eval, print the result.
 * ================================================================= */
main:
    push    %rbx                /* entry rsp%16==8; this push -> 0, aligned for calls below */

    lea     heap(%rip), %rax    /* heap_ptr starts unset (.bss is zeroed) -- point it at heap */
    mov     %rax, heap_ptr(%rip)

    /* ---- Trace 1: e1 = (CONS (QUOTE A) (QUOTE B)) ---- */
    mov     $A_SYM, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (A) */
    mov     $QUOTE_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* qA = (QUOTE A) */
    mov     %rax, %r12           /* r12 = qA */

    mov     $B_SYM, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (B) */
    mov     $QUOTE_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* qB = (QUOTE B) */
    mov     %rax, %r13           /* r13 = qB */

    mov     %r13, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (qB) */
    mov     %r12, %rdi
    mov     %rax, %rsi
    call    cons                 /* (qA qB) */
    mov     $CONS_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* e1 = (CONS qA qB) */

    mov     %rax, %rdi
    mov     $NIL_SYM, %rsi
    call    eval                 /* result1 = eval[e1;NIL] */
    mov     %rax, %rdi
    call    print_pair           /* prints "(A . B)" */

    /* ---- Trace 2: e2 = ((LAMBDA (X) (CONS X X)) (QUOTE A)) ---- */
    mov     $X_SYM, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* params = (X) */
    mov     %rax, %r12

    mov     $X_SYM, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (X) */
    mov     $X_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* (X X) */
    mov     $CONS_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* body = (CONS X X) */
    mov     %rax, %r13

    mov     %r13, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (body) */
    mov     %r12, %rdi
    mov     %rax, %rsi
    call    cons                 /* (params body) */
    mov     $LAMBDA_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* lambda_expr = (LAMBDA params body) */
    mov     %rax, %r14

    mov     $A_SYM, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (A) */
    mov     $QUOTE_SYM, %rdi
    mov     %rax, %rsi
    call    cons                 /* arg1 = (QUOTE A) */

    mov     %rax, %rdi
    mov     $NIL_SYM, %rsi
    call    cons                 /* (arg1) */
    mov     %r14, %rdi
    mov     %rax, %rsi
    call    cons                 /* e2 = (lambda_expr arg1) */

    mov     %rax, %rdi
    mov     $NIL_SYM, %rsi
    call    eval                 /* result2 = eval[e2;NIL] */
    mov     %rax, %rdi
    call    print_pair           /* prints "(A . A)" */

    pop     %rbx
    xor     %eax, %eax
    ret

    .section .note.GNU-stack,"",@progbits
