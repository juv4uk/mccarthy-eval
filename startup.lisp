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

(DEFINE NULL
  (LAMBDA (X)
    (EQ X NIL)))

(DEFINE FF
  (LABEL FF
    (LAMBDA (X)
      (COND ((ATOM X) X)
            (T (FF (CAR X)))))))

(DEFINE SUBST
  (LABEL SUBST
    (LAMBDA (X Y Z)
      (COND ((ATOM Z)
             (COND ((EQ Z Y) X)
                   (T Z)))
            (T (CONS (SUBST X Y (CAR Z))
                     (SUBST X Y (CDR Z))))))))

(DEFINE EQUAL
  (LABEL EQUAL
    (LAMBDA (X Y)
      (COND ((ATOM X)
             (COND ((ATOM Y) (EQ X Y))
                   (T NIL)))
            ((EQUAL (CAR X) (CAR Y))
             (EQUAL (CDR X) (CDR Y)))
            (T NIL)))))

(DEFINE MEMBER
  (LABEL MEMBER
    (LAMBDA (X Y)
      (COND ((NULL Y) NIL)
            ((EQUAL X (CAR Y)) T)
            (T (MEMBER X (CDR Y)))))))
