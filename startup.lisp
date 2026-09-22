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

(DEFINE PAIR
  (LABEL PAIR
    (LAMBDA (X Y)
      (COND ((NULL X) NIL)
            (T (CONS (CONS (CAR X) (CAR Y)) (PAIR (CDR X) (CDR Y))))))))

(DEFINE SASSOC
  (LABEL SASSOC
    (LAMBDA (X Y U)
      (COND ((NULL Y) (U))
            ((EQ X (CAR (CAR Y))) (CAR Y))
            (T (SASSOC X (CDR Y) U))))))

(DEFINE SEARCH
  (LABEL SEARCH
    (LAMBDA (X P F U)
      (COND ((NULL X) (U X))
            ((P (CAR X)) (F (CAR X)))
            (T (SEARCH (CDR X) P F U))))))

(DEFINE REVERSE-ACC
  (LABEL REVERSE-ACC
    (LAMBDA (L ACC)
      (COND ((NULL L) ACC)
            (T (REVERSE-ACC (CDR L) (CONS (CAR L) ACC)))))))

(DEFINE REVERSE
  (LAMBDA (L)
    (REVERSE-ACC L NIL)))

(DEFINE LENGTH
  (LABEL LENGTH
    (LAMBDA (X)
      (COND ((NULL X) 0)
            (T (ADD1 (LENGTH (CDR X))))))))

(DEFINE COPY
  (LABEL COPY
    (LAMBDA (X)
      (COND ((NULL X) NIL)
            ((ATOM X) X)
            (T (CONS (COPY (CAR X)) (COPY (CDR X))))))))

(DEFINE SUBLIS-SCAN
  (LABEL SUBLIS-SCAN
    (LAMBDA (ALIST FULLX Y)
      (COND ((NULL ALIST)
             (COND ((ATOM Y) Y)
                   (T (CONS (SUBLIS-SCAN FULLX FULLX (CAR Y))
                            (SUBLIS-SCAN FULLX FULLX (CDR Y))))))
            ((EQUAL Y (CAR (CAR ALIST))) (CDR (CAR ALIST)))
            (T (SUBLIS-SCAN (CDR ALIST) FULLX Y))))))

(DEFINE SUBLIS
  (LAMBDA (X Y)
    (COND ((NULL X) Y)
          (T (SUBLIS-SCAN X X Y)))))
