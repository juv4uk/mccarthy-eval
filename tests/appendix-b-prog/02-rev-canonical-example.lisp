(DEFINE REV
  (LABEL REV
    (LAMBDA (X)
      (PROG (Y Z)
        A (COND ((NULL X) (RETURN Y)))
        (SETQ Z (CAR X))
        (COND ((ATOM Z) (GO B)))
        (SETQ Z (REV Z))
        B (SETQ Y (CONS Z Y))
        (SETQ X (CDR X))
        (GO A)))))
(REV (QUOTE (A ((B C) D))))
