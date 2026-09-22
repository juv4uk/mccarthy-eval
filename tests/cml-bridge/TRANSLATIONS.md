# CML bridge source translations

## Fixture 19 — LABEL recursion

Historical source:

`tests/historical-core/19-label-recursion-mylen.lisp`

Historical form:

```lisp
((LABEL MYLEN
   (LAMBDA (LST)
     (COND ((EQ LST NIL) NIL)
           (T (CONS T (MYLEN (CDR LST)))))))
 (QUOTE (A B C)))
```

Witness translation:

```lisp
(def MYLEN
  (lambda (LST)
    (cond
      ((eq LST NIL) NIL)
      (T (cons T (MYLEN (cdr LST)))))))

(MYLEN (quote (A B C)))
```

The transformation is intentionally explicit:

```
LABEL name + LAMBDA
→
named self-recursive def/lambda
```

### Why this translation is currently admissible as a witness candidate

Current CML documents/tests support self-recursive named `def` through a
placeholder/backpatch mechanism. The historical fixture uses one recursive
binding; it does not require a mutually recursive forward definition.

The translation must still be executed and compared before fixture 19 is
marked `match`. The current expected historical output is `(T T T)`; that
value belongs to Witness A and must not be copied into CML/Rust as an
expected answer.

### What this translation does NOT prove

It does not prove that arbitrary McCarthy `LABEL` semantics are equivalent
to modern CML `def` semantics.

In particular, fixture 20 remains blocked because its nested `LABEL` tests
environment shadowing:

```
outer LABEL X
  → inner LABEL X
```

A global/self-recursive `def` rewrite is not accepted as proof for that
shape.

Status for #13:
- fixture 19: **translation prepared; execution evidence pending**
- fixture 20: **blocked**
- fixture 21: **blocked pending appq/value-vs-code witness**
