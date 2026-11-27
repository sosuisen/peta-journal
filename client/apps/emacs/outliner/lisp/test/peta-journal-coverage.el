;;; peta-journal-coverage.el --- Counts the lines of the mode that the tests run -*- lexical-binding: t; -*-

;;; Commentary:

;; Loaded before the mode and the tests by coverage.mjs, which fetches undercover.el, reads the
;; report and fails under the floor.  Every file of lisp/ is measured: none is left out.

;;; Code:

(require 'undercover)

;; Without it, undercover measures only on the machines of a CI
(setq undercover-force-coverage t)

(undercover "lisp/*.el"
            (:report-format 'lcov)
            (:report-file (getenv "PETA_COVERAGE_REPORT"))
            (:send-report nil))

(provide 'peta-journal-coverage)
;;; peta-journal-coverage.el ends here
