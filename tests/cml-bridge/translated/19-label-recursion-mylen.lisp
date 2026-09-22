; Generated source for CML bridge witness #13.
; Historical source: tests/historical-core/19-label-recursion-mylen.lisp
; Translation under test:
;   ((LABEL F (LAMBDA ...)) ARG)
; =>
;   (def F (lambda ...))
;   (F ARG)
;
; This is NOT asserted to be a historical source form. It is a compiler-witness
; transformation whose preservation must be demonstrated before fixture 19 is
; admitted to the full CML differential matrix.

(def MYLEN
  (lambda (LST)
    (cond
      ((eq LST NIL) NIL)
      (T (cons T (MYLEN (cdr LST)))))))

(MYLEN (quote (A B C)))
