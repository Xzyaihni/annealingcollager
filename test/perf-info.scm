(define (list-prefix? prefix lst)
    (cond
        ((null? prefix) #t)
        ((null? lst) #f)
        (else
            (and
                (char=? (car prefix) (car lst))
                (list-prefix? (cdr prefix) (cdr lst))))))

(define (string-prefix? prefix str)
    (list-prefix? (string->list prefix) (string->list str)))

(define (list-skip lst n)
    (if (<= n 0)
        lst
        (list-skip (cdr lst) (- n 1))))

(define (string-split s delim)
    (define (list-split lst delim current)
        (cond
            ((null? lst) (list current))
            ((list-prefix? delim lst)
                (cons
                    current
                    (list-split
                        (list-skip lst (length delim))
                        delim
                        '())))
            (else
                (list-split
                    (cdr lst)
                    delim
                    (append current (list (car lst)))))))
    (map
        list->string
        (list-split (string->list s) (string->list delim) '())))

(define (parse-line line)
    (cond
        ((string-prefix? "progress" line) '())
        ((string-prefix? "final error" line)
            (string->number
                (cadr (string-split line ": "))))
        (else (error "what: " line))))

(define (read-file port)
    (define (read-raw port)
        (let ((value (read-line port)))
            (if (eof-object? value)
                '()
                (cons (parse-line value) (read-raw port)))))
    (filter (lambda (x) (not (null? x))) (read-raw port)))

(define (average-lst lst)
    (/ (fold-right + 0 lst) (length lst)))

(define (standard-deviation lst)
    (let ((avg (average-lst lst)))
        (sqrt
            (/
                (fold-right
                    +
                    0
                    (map (lambda (x) (square (- x avg))) lst))
                (- (length lst) 1)))))

(define (round-places value places)
    (let ((mag (expt 10 places)))
        (/ (round (* value mag)) mag)))

(define (make-population path)
    (let ((vals (read-file (open-input-file path))))
        (list vals (length vals) (average-lst vals) (standard-deviation vals))))

(define (vals-population p)
    (car p))

(define (length-population p)
    (cadr p))

(define (average-population p)
    (caddr p))

(define (sdev-population p)
    (cadddr p))

(define (serror-population p)
    (/
        (square (sdev-population p))
        (length-population p)))

(define (zvalue-populations p0 p1)
    (/
        (- (average-population p0) (average-population p1))
        (sqrt
            (+
                (serror-population p0)
                (serror-population p1)))))

(define (erf x)
    (tanh
        (*
            (/
                2
                (sqrt 3.1415926535897932))
            (+
                x
                (*
                    (/ 11 123)
                    (* x x x))))))

(define (pvalue-populations p0 p1)
    (*
        (/ 1 2)
        (+
            1
            (erf
                (/
                    (zvalue-populations p0 p1)
                    (sqrt 2))))))

(define (make-bound low high)
    (cons low high))

(define (low-bound b)
    (car b))

(define (high-bound b)
    (cdr b))

(define (bounds-population p)
    (let ((avg (average-population p))
            (dev (sdev-population p)))
        (make-bound (- avg dev) (+ avg dev))))

(define (print-bounds p)
    (let ((b (bounds-population p)))
        (display (round-places (low-bound b) 1))
        (display " - ")
        (display (round-places (high-bound b) 1))))

(define args (cdr (command-line)))

(cond
    ((= (length args) 1)
        (let ((p0 (make-population (car args))))
            (display "95% of samples fall between: ")
            (print-bounds p0)
            (newline)
            (display "standard deviation: ")
            (display (round-places dev 2))
            (newline)))
    ((= (length args) 2)
        (let ((p0 (make-population (car args)))
                (p1 (make-population (cadr args))))
            (display "95% of samples fall between: ")
            (newline)
            (display "for left: ")
            (print-bounds p0)
            (newline)
            (display "for right: ")
            (print-bounds p1)
            (newline)
            (display "z value: ")
            (display (round-places (zvalue-populations p0 p1) 3))
            (newline)
            (display "p value: ")
            (display (round-places (pvalue-populations p0 p1) 3))
            (newline)))
    (else (error "too many args: " args)))
