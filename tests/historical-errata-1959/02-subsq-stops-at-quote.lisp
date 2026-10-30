(DEFINE SUBSQ
  (LABEL SUBSQ
    (LAMBDA (X Y Z)
      (COND ((NULL Z) NIL)
            ((ATOM Z) (COND ((EQ Y Z) X) (T Z)))
            ((EQ (CAR Z) (QUOTE QUOTE)) Z)
            (T (CONS (SUBSQ X Y (CAR Z))
                     (SUBSQ X Y (CDR Z))))))))
(SUBSQ (QUOTE A) (QUOTE X) (QUOTE (QUOTE X)))
