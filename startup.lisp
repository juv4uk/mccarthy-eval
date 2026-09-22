(DEFINE SQ
  (LAMBDA (X)
    (TIMES X X)))

(DEFINE DOUBLE
  (LAMBDA (X)
    (PLUS X X)))

(DEFINE MYAPPEND
  (LABEL MYAPPEND
    (LAMBDA (X Y)
      (COND ((EQ X NIL) Y)
            (T (CONS (CAR X) (MYAPPEND (CDR X) Y)))))))

(DEFINE FF
  (LABEL FF
    (LAMBDA (X)
      (COND ((ATOM X) X)
            (T (FF (CAR X)))))))

(DEFINE SUBST
  (LABEL SUBST
    (LAMBDA (X Y Z)
      (COND ((EQUAL Y Z) X)
            ((ATOM Z) Z)
            (T (CONS (SUBST X Y (CAR Z))
                     (SUBST X Y (CDR Z))))))))

(DEFINE MEMBER
  (LABEL MEMBER
    (LAMBDA (X Y)
      (COND ((NULL Y) NIL)
            ((EQUAL X (CAR Y)) T)
            (T (MEMBER X (CDR Y)))))))

(DEFINE APPEND
  (LABEL APPEND
    (LAMBDA (X Y)
      (COND ((NULL X) Y)
            (T (CONS (CAR X) (APPEND (CDR X) Y)))))))

(DEFINE MAPLIST
  (LABEL MAPLIST
    (LAMBDA (X F)
      (COND ((NULL X) NIL)
            (T (CONS (F X) (MAPLIST (CDR X) F)))))))
