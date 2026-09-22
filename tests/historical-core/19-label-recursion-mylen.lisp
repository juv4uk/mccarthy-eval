((LABEL MYLEN
   (LAMBDA (LST)
     (COND ((EQ LST NIL) NIL)
           (T (CONS T (MYLEN (CDR LST)))))))
 (QUOTE (A B C)))
