;;; format.el --- The formatter of the Emacs Lisp of the shell -*- lexical-binding: t; -*-

;;; Commentary:

;; Emacs Lisp has no formatter of its own, so Emacs indents it.  The rule is the indent of
;; `emacs-lisp-mode', with spaces and no tabs, and no space at the end of a line.
;;
;;   emacs -Q --batch -l format.el -f peta-format-write   ; formats the files
;;   emacs -Q --batch -l format.el -f peta-format-check   ; changes nothing, fails on a change
;;
;; Run it from any folder: it finds the files under the lisp folder next to it.  The lines of
;; peta-journal-protocol.el come from Rust, which writes them in the same form.

;;; Code:

(defconst peta-format--root (file-name-directory (or load-file-name buffer-file-name))
  "The folder of the shell, where this file is.")

(defun peta-format--files ()
  "Return the Emacs Lisp files of the shell, this file among them."
  (cons (expand-file-name "format.el" peta-format--root)
        (directory-files-recursively (expand-file-name "lisp" peta-format--root) "\\.el\\'")))

(defun peta-format--formatted (file)
  "Return the text of FILE as the formatter writes it."
  (with-temp-buffer
    (insert-file-contents file)
    (emacs-lisp-mode)
    (setq indent-tabs-mode nil)
    (let ((inhibit-message t))
      (indent-region (point-min) (point-max)))
    (delete-trailing-whitespace)
    (buffer-string)))

(defun peta-format--text (file)
  "Return the text of FILE as it is now."
  (with-temp-buffer
    (insert-file-contents file)
    (buffer-string)))

(defun peta-format--changed ()
  "Return the files the formatter would change, each with its formatted text."
  (let (changed)
    (dolist (file (peta-format--files) (nreverse changed))
      (let ((formatted (peta-format--formatted file)))
        (unless (string= formatted (peta-format--text file))
          (push (cons file formatted) changed))))))

(defun peta-format-write ()
  "Format the Emacs Lisp files of the shell."
  (dolist (entry (peta-format--changed))
    (let ((coding-system-for-write 'utf-8-unix))
      (with-temp-file (car entry)
        (insert (cdr entry))))
    (message "formatted %s" (file-relative-name (car entry) peta-format--root))))

(defun peta-format-check ()
  "Fail when a file of the shell is not formatted."
  (let ((changed (peta-format--changed)))
    (dolist (entry changed)
      (message "not formatted: %s" (file-relative-name (car entry) peta-format--root)))
    (when changed
      (message "Run: emacs -Q --batch -l format.el -f peta-format-write")
      (kill-emacs 1))
    (message "All %d files are formatted." (length (peta-format--files)))))

;;; format.el ends here
