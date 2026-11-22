;;; peta-journal-timing.el --- How long Emacs takes to show the rows -*- lexical-binding: t; -*-

;;; Commentary:

;; Prints how long the buffer takes to read a line of rows from the shell and to show it, on
;; pages of every length (#859).  It checks nothing, so it is not one of the tests.  Run it with:
;;   emacs -Q --batch -L lisp -l lisp/test/peta-journal-timing.el -f peta-journal-timing-run
;;
;; The lines are those the shell writes when one row changes: the whole list, with the last
;; row one level deeper or not.  A batch Emacs does not draw, so the time of the display is
;; not in here.

;;; Code:

(require 'cl-lib)
(require 'peta-journal)

(defconst peta-journal-timing--lengths '(1 10 100 1000))
(defconst peta-journal-timing--keys 100)

(defun peta-journal-timing--row (n length deeper)
  "Row N of a page of LENGTH items, as the shell writes it.
The last row is one level deeper when DEEPER is non-nil."
  (list :id (format "%04d" n)
        :level (+ (% n 3) 1 (if (and deeper (= n (1- length))) 1 0))
        :body (format "**item** %d" n)
        :text (list :text (format "item %d\n" n)
                    :spans (vector (list :from 0 :to 4 :looks ["bold"] :link :null
                                         :image :null :letters t)))
        :bullet "plain" :task :null :collapsed :false
        :header :null :selected :false))

(defun peta-journal-timing--line (changes)
  "The line of the CHANGES of the rows, as the shell writes it."
  (concat (json-serialize (list :t "rows" :ack 0 :page_id "p" :changes (vconcat changes)))
          "\n"))

(defun peta-journal-timing--reset (length)
  "The line that shows a page of LENGTH items."
  (peta-journal-timing--line
   (list (list :Reset (list :items (vconcat (cl-loop for n below length
                                                     collect (peta-journal-timing--row
                                                              n length nil))))))))

(defun peta-journal-timing--update (length deeper)
  "The line that changes the level of the last row of a page of LENGTH items.
The row is one level deeper when DEEPER is non-nil."
  (let ((last (1- length)))
    (peta-journal-timing--line
     (list (list :Update (list :at last
                               :item (peta-journal-timing--row last length deeper)))))))

(defun peta-journal-timing--median (times)
  "The middle of TIMES."
  (nth (/ (length times) 2) (sort times #'<)))

(defun peta-journal-timing-run ()
  "Print the time to read and show one line of rows, for each length."
  (dolist (length peta-journal-timing--lengths)
    (with-temp-buffer
      (peta-journal-mode)
      (cl-letf (((symbol-function 'peta-journal--send) #'ignore))
        (let ((lines (vector (peta-journal-timing--update length nil)
                             (peta-journal-timing--update length t)))
              (times nil))
          (peta-journal--read-output (peta-journal-timing--reset length))
          (garbage-collect)
          (dotimes (n peta-journal-timing--keys)
            (let ((line (aref lines (% (1+ n) 2)))
                  (start (float-time)))
              (peta-journal--read-output line)
              (push (* 1000 (- (float-time) start)) times)))
          (princ (format "rows %5d items: %8.3fms, %7d bytes\n"
                         length (peta-journal-timing--median times)
                         (string-bytes (aref lines 1)))))))))

(provide 'peta-journal-timing)
;;; peta-journal-timing.el ends here
