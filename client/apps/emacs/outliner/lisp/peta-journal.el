;;; peta-journal.el --- Emacs frontend for PetaJournal -*- lexical-binding: t; -*-

;; Package-Requires: ((emacs "29.1"))

;;; Commentary:

;; A thin view on the PetaJournal core.  The program `client-shell-emacs' holds the core and talks
;; to the journal server.  This file draws the rows it is sent and tells it what the user does.
;; Saving, undo and the rules of the outline all live in the core.
;;
;; The whole page is in one buffer.  A row is a bullet, the body and a newline.  The body is plain
;; text to edit.  The bullet and the newline are read-only, as the outline only changes through
;; commands such as `peta-journal-enter' and `peta-journal-indent'.
;;
;; M-x peta-journal opens today's journal.

;;; Code:

(require 'calendar)
(require 'cl-lib)
(require 'parse-time)
(require 'subr-x)
(require 'text-property-search)

(defgroup peta-journal nil
  "Emacs frontend for PetaJournal."
  :group 'applications)

(defcustom peta-journal-server "http://localhost:8087"
  "The address of the journal server."
  :type 'string)

(defcustom peta-journal-shell-program nil
  "The program `client-shell-emacs'.
When nil, it is looked for on the `exec-path' and in the build folder
of the repository."
  :type '(choice (const nil) file))

(defcustom peta-journal-idle-finish-seconds 2
  "How long Emacs may idle before the edit of the current item ends.
The core does not take in changes from elsewhere while an edit is open."
  :type 'number)

(defcustom peta-journal-item-gap 4
  "The gap between two items, in pixels.
It applies to the rows drawn after a change.  Only a graphical Emacs shows it."
  :type 'natnum)

(defconst peta-journal--bullets
  '(("plain" . "•") ("transclusion" . "⊡") ("broken" . "✗"))
  "The mark of each kind of row.
A transclusion has a square around its dot, as the web frontend draws it.")

(defconst peta-journal--collapsed-bullets
  '(("plain" . "◉") ("transclusion" . "▣"))
  "The mark of each kind of row whose children are hidden.
A ring around the dot, and a filled square for a transclusion, as the web
frontend draws them.  A kind that is not here keeps its mark.")

(defconst peta-journal--task-marks
  '(("task" . "☐") ("running" . "⊟") ("done" . "☑"))
  "The mark of each state of a task.
It comes behind the bullet: a task has the bullet of its kind too, as in the
web frontend.")

(defconst peta-journal--bullet-gap " "
  "The space behind a bullet.
A no-break space: at a normal space, a long word would leave the bullet alone
on its line.")

(defconst peta-journal--wait-seconds 3
  "How long a command waits for the shell before the next key is handled.")

(defconst peta-journal--stop-seconds 6
  "How long Emacs waits for the shell to save what was typed when it stops.")

(defface peta-journal-bullet '((t :inherit font-lock-comment-face))
  "The mark at the start of a row.")

(defface peta-journal-done '((t :inherit font-lock-string-face))
  "The mark of a finished task.")

(defface peta-journal-running '((t :inherit peta-journal-bullet :height 1.7))
  "The mark of a running task.
Larger than the text: in many fonts the box with a line is smaller than the
boxes of the other states.")

(defface peta-journal-date-header '((t :inherit font-lock-doc-face :weight bold))
  "The date above the first item of a journal day.")

(defface peta-journal-link '((t :inherit link))
  "A link to a page.")

(defface peta-journal-heading '((t :inherit font-lock-function-name-face :weight bold))
  "A body that is a heading.")

(defface peta-journal-bold '((t :inherit font-lock-keyword-face :weight bold))
  "Bold text.")

(defface peta-journal-italic '((t :inherit font-lock-variable-name-face :slant italic))
  "Italic text.")

(defface peta-journal-underline '((t :inherit font-lock-string-face :underline t))
  "Underlined text.")

(defface peta-journal-strike '((t :inherit font-lock-comment-face :strike-through t))
  "Text with a line through it.")

(defface peta-journal-script '((t :inherit font-lock-constant-face))
  "Superscript and subscript text.")

(defface peta-journal-highlight '((t :inherit match))
  "Highlighted text.")

(defface peta-journal-spoiler '((t :inherit shadow))
  "Spoiler text.")

(defface peta-journal-code '((t :inherit font-lock-constant-face))
  "Inline code and code blocks.")

(defface peta-journal-math '((t :inherit font-lock-builtin-face))
  "Inline math.")

(defface peta-journal-quote '((t :inherit font-lock-doc-face))
  "A block quote.")

(defface peta-journal-alert '((t :inherit font-lock-warning-face))
  "The title line of an alert.")

(defface peta-journal-list-mark '((t :inherit font-lock-builtin-face))
  "The mark of a list item or of a task in a list.")

(defface peta-journal-rule '((t :inherit shadow))
  "A horizontal rule.")

(defface peta-journal-subtext '((t :inherit shadow))
  "Small text under a block.")

(defface peta-journal-table '((t :inherit font-lock-type-face))
  "A table.")

(defface peta-journal-image '((t :inherit font-lock-function-name-face))
  "An image or another embedded file.")

(defface peta-journal-footnote '((t :inherit font-lock-builtin-face))
  "A footnote.")

(defface peta-journal-emoji '((t :inherit font-lock-string-face))
  "An emoji shortcode.")

(defface peta-journal-html '((t :inherit font-lock-preprocessor-face))
  "An HTML tag.")

;;;; Finding Flat Markdown

;; These rules follow the parser of the server only roughly.  The shell sends Emacs the text of a
;; body and no parsed spans, and asking for them on each key would slow typing down.

(defun peta-journal--word-char-p (char)
  "Non-nil when CHAR is a letter or a digit.  CHAR is nil at the buffer edge."
  (and char (string-match-p "[[:alnum:]]" (string char))))

(defun peta-journal--delimiters-fit-p (mark start end)
  "Non-nil when the pair of MARK from START to END can decorate its text.
A regexp cannot check this.  It would take the characters around the pair
into its match, and the next pair could not start at a character it took."
  (let ((before (char-before start))
        (after (char-after end)))
    (and (not (memq before (list mark ?\\)))
         (not (eq after mark))
         (or (not (eq mark ?_))
             (not (or (peta-journal--word-char-p before)
                      (peta-journal--word-char-p after)))))))

(defun peta-journal--fitting-pair (regexp mark)
  "A font-lock matcher for REGEXP, when its ends are delimiters of MARK that fit."
  (lambda (limit)
    (let (found)
      (while (and (not found) (re-search-forward regexp limit t))
        (if (peta-journal--delimiters-fit-p mark (match-beginning 0) (match-end 0))
            (setq found t)
          (goto-char (1+ (match-beginning 0)))))
      found)))

(defun peta-journal--pair (delimiter)
  "A font-lock matcher for text between two DELIMITERs on one line."
  (let ((quoted (regexp-quote delimiter)))
    (peta-journal--fitting-pair (concat quoted "[^ \t\n]\\(?:.*?[^ \t\n]\\)??" quoted)
                                (aref delimiter 0))))

(defconst peta-journal--bullet-regexp
  (concat (regexp-opt (mapcar #'cdr (append peta-journal--bullets
                                            peta-journal--collapsed-bullets)))
          peta-journal--bullet-gap
          "\\(?:" (regexp-opt (mapcar #'cdr peta-journal--task-marks))
          peta-journal--bullet-gap "\\)?")
  "The bullet in front of the first line of a body, with the mark of a task.
A body that starts with such a mark and a no-break space reads as a task here.
That only moves where the colors of its first line start.")

(defconst peta-journal--line-start-regexp
  (concat "^\\(?:" peta-journal--bullet-regexp "\\)?")
  "What comes before the text of a line in a body.")

(defun peta-journal--fence (opening)
  "A font-lock matcher for a block that starts at a line matching OPENING.
Group 1 of OPENING is the run of marks that the closing line repeats.
A block with no closing ends with its body, since each item is parsed alone."
  (let ((regexp (concat peta-journal--line-start-regexp " \\{0,3\\}" opening)))
    (lambda (limit)
      (when (re-search-forward regexp limit t)
        (let* ((run (match-string 1))
               (start (match-beginning 1))
               (end (peta-journal--body-end (peta-journal--row-start (point))))
               (closing (format "^ \\{0,3\\}%s%s* *$"
                                (regexp-quote run)
                                (regexp-quote (substring run 0 1)))))
          (unless (re-search-forward closing end t)
            (goto-char end))
          (set-match-data (list start (point)))
          t)))))

(defun peta-journal--match-alert (limit)
  "Find the title line of an alert before LIMIT.  Its kind may be in any case."
  (let ((case-fold-search t))
    (re-search-forward
     (concat peta-journal--line-start-regexp
             " \\{0,3\\}\\(>\\(?:>>\\)? *\\[!\\(?:note\\|tip\\|important\\|warning\\|caution\\)\\].*\\)$")
     limit t)))

(defun peta-journal--table-start ()
  "The start of the head line above the line at the point, behind any bullet."
  (save-excursion
    (forward-line -1)
    (if (looking-at peta-journal--bullet-regexp)
        (match-end 0)
      (point))))

(defun peta-journal--table-end (body-end)
  "The end of the last table line from the point, not going past BODY-END."
  (save-excursion
    (end-of-line)
    (while (and (< (point) body-end)
                (save-excursion
                  (forward-line 1)
                  (string-search "|" (buffer-substring (point) (line-end-position)))))
      (forward-line 1)
      (end-of-line))
    (point)))

(defun peta-journal--match-table (limit)
  "Find a table before LIMIT, from the line above a delimiter line.
A line of cells alone is plain text, and `---' with no `|' is not a delimiter."
  (let (found)
    (while (and (not found)
                (re-search-forward "^ *|? *:?-+:? *\\(?:| *:?-+:? *\\)*|? *$" limit t))
      (when (string-search "|" (match-string 0))
        (let ((start (peta-journal--table-start))
              (end (peta-journal--table-end
                    (peta-journal--body-end (peta-journal--row-start (point))))))
          (goto-char end)
          (set-match-data (list start end))
          (setq found t))))
    found))

(defconst peta-journal--font-lock-keywords
  `(;; Code comes first: what has a face stays as it is, so nothing is decorated inside
    (,(peta-journal--fence "\\(```+\\)[^`\n]*$") 0 'peta-journal-code)
    (,(peta-journal--fence "\\(~~~+\\).*$") 0 'peta-journal-code)
    (,(concat "^" peta-journal--bullet-regexp "\\( \\{4\\}.*\\(?:\n \\{4\\}.*\\)*\\)$")
     1 'peta-journal-code)
    ;; Before the rules: the `---' under a title is not a rule
    (,(concat peta-journal--line-start-regexp "\\([^-+>|# \n][^\n]*\n *\\(?:=+\\|-+\\) *\\)$")
     1 'peta-journal-heading)
    ;; Before the emphasis: `***' would be an italic `*'
    (,(concat peta-journal--line-start-regexp " \\{0,3\\}\\(\\([-*_]\\)\\(?: *\\2\\)\\{2,\\} *\\)$")
     1 'peta-journal-rule)
    ("\\$`[^`\n]+`\\$" 0 'peta-journal-math)
    (,(peta-journal--fitting-pair "\\(`+\\)[^`\n]\\(?:.*?[^`\n]\\)??\\1" ?`)
     0 'peta-journal-code)
    ;; Links and the like come before the emphasis: `_' in a URL is not emphasis
    ("\\[\\[[^]\n]+\\]\\]" 0 'peta-journal-link)
    (,(concat "\\(?:^\\|[ \t" peta-journal--bullet-gap "]\\)"
              "\\(#[[:alnum:]_-]\\(?:[[:alnum:]_/-]*[[:alnum:]_-]\\)?\\)")
     1 'peta-journal-link)
    ("!\\[[^][\n]*\\]([^)\n]*)" 0 'peta-journal-image)
    ("\\[[^][\n]*\\]([^)\n]*)" 0 'peta-journal-link)
    ("\\(?:https?://\\|www\\.\\)[^ \t\n<>]+" 0 'peta-journal-link)
    ("[[:alnum:]._+-]+@[[:alnum:]-]+\\.[[:alnum:].-]+" 0 'peta-journal-link)
    ("\\[\\^[^]\n]+\\]\\|\\^\\[[^]\n]*\\]" 0 'peta-journal-footnote)
    ("\\(?:^\\|[^[:alnum:]]\\)\\(:\\(?:[-+]1\\|[a-z][a-z0-9_+-]*\\):\\)" 1 'peta-journal-emoji)
    ("</?[A-Za-z][A-Za-z0-9-]*\\(?:[ \t/][^<>\n]*\\)?>" 0 'peta-journal-html)
    ;; The longer delimiters come first: `**' also holds two `*'
    (,(peta-journal--pair "**") 0 'peta-journal-bold keep)
    (,(peta-journal--pair "__") 0 'peta-journal-underline keep)
    (,(peta-journal--pair "~~") 0 'peta-journal-strike keep)
    (,(peta-journal--pair "==") 0 'peta-journal-highlight keep)
    (,(peta-journal--pair "||") 0 'peta-journal-spoiler keep)
    (,(peta-journal--pair "*") 0 'peta-journal-italic keep)
    (,(peta-journal--pair "_") 0 'peta-journal-italic keep)
    (,(peta-journal--pair "~") 0 'peta-journal-script keep)
    (,(peta-journal--pair "^") 0 'peta-journal-script keep)
    ;; Blocks come after the inline rules, and fill in what they left
    (,(concat peta-journal--line-start-regexp " \\{0,3\\}\\(#\\{1,6\\}\\(?: .*\\)?\\)$")
     1 'peta-journal-heading keep)
    (peta-journal--match-table 0 'peta-journal-table keep)
    (,(concat peta-journal--line-start-regexp " \\{0,3\\}\\(-# .*\\)$") 1 'peta-journal-subtext keep)
    (,(concat peta-journal--line-start-regexp
              " *\\([-+*]\\(?: \\[[^]\n]\\]\\)?\\|[0-9]+[.)]\\)\\(?: \\|$\\)")
     1 'peta-journal-list-mark keep)
    (peta-journal--match-alert 1 'peta-journal-alert keep)
    (,(peta-journal--fence "\\(>>>+\\).*$") 0 'peta-journal-quote keep)
    (,(concat peta-journal--line-start-regexp " \\{0,3\\}\\(>.*\\)$") 1 'peta-journal-quote keep))
  "What is highlighted in a body.")

;;;; State

(defvar peta-journal--recent nil
  "The paths of the recent pages, the most recently modified first.")

(defun peta-journal--recent-pages ()
  "The recent pages as a completion table that keeps their order.
Completion sorts a plain list by name, which would hide the newest pages."
  (let ((pages peta-journal--recent))
    (lambda (string predicate action)
      (if (eq action 'metadata)
          '(metadata (display-sort-function . identity)
                     (cycle-sort-function . identity))
        (complete-with-action action pages string predicate)))))
(defvar peta-journal--idle-timer nil "The timer that ends an edit after an idle moment.")

;; Each buffer has a shell of its own, as one core shows one page
(defvar-local peta-journal--process nil "The running shell.")
(defvar-local peta-journal--seq 0 "The number of the last line sent to the shell.")
(defvar-local peta-journal--ack 0 "The number of the last line the shell has handled.")
(defvar-local peta-journal--holding nil "Non-nil while the core waits for an item to focus.")
(defvar-local peta-journal--output "" "The part of a line from the shell that is not complete yet.")
(defvar-local peta-journal--backlinks nil "The pages that link to the shown page.")
(defvar-local peta-journal--scroll-timer nil "The timer that reports the scroll position.")
(defvar-local peta-journal--rows [] "The rows on screen, as the shell sent them.")
(defvar-local peta-journal--page-id nil "The id of the page on screen.")
(defvar-local peta-journal--current nil "The id of the item being edited.")
(defvar-local peta-journal--last-point nil "Where the point was after the last command.")
(defvar-local peta-journal--title "" "The title of the page on screen.")
(defvar-local peta-journal--page-path nil "The path of the page on screen, if it can be renamed.")
(defvar-local peta-journal--properties nil
  "The property rows of the page on screen, or nil on a screen without properties.")
(defvar-local peta-journal--free-keys nil "The property keys that can still be added to the page.")
(defvar-local peta-journal--datetime-keys nil "The property keys whose value is a date and a time.")
(defvar-local peta-journal--connection "" "The state of the connection to the server.")
(defvar-local peta-journal--scrolled nil "The scroll position the core knows about.")
(defvar-local peta-journal--page-names nil
  "The last page names the shell found: a cons of what was asked and the names.")

;;;; Finding rows

(defun peta-journal--row-start (pos)
  "The start of the bullet of the row at POS, or nil when there is no row there."
  (save-excursion
    (goto-char pos)
    (beginning-of-line)
    (when (get-text-property (point) 'peta-header)
      (forward-line 1))
    (while (and (not (get-text-property (point) 'peta-bullet))
                (not (bobp)))
      (forward-line -1))
    (and (get-text-property (point) 'peta-bullet) (point))))

(defun peta-journal--body-start (row-start)
  "The start of the body of the row with its bullet at ROW-START.
The bullet says how long the marks in front of the body are."
  (+ row-start (get-text-property row-start 'peta-marks)))

(defun peta-journal--body-end (row-start)
  "The end of the body of the row with its bullet at ROW-START, before its newline."
  (save-excursion
    (goto-char row-start)
    (forward-line 1)
    (while (and (not (eobp))
                (not (get-text-property (point) 'peta-bullet))
                (not (get-text-property (point) 'peta-header)))
      (forward-line 1))
    (1- (point))))

(defun peta-journal--id-at (pos)
  "The id of the item at POS, or nil."
  (when-let* ((start (peta-journal--row-start pos)))
    (get-text-property start 'peta-bullet)))

(defun peta-journal--body-at (pos)
  "The body of the item at POS."
  (let ((start (peta-journal--row-start pos)))
    (buffer-substring-no-properties (peta-journal--body-start start)
                                    (peta-journal--body-end start))))

(defun peta-journal--cursor ()
  "The place of the point in its body, in characters."
  (- (point) (peta-journal--body-start (peta-journal--row-start (point)))))

(defun peta-journal--find-row (id)
  "The start of the bullet of the item ID, or nil."
  (save-excursion
    (goto-char (point-min))
    (when-let* ((match (text-property-search-forward 'peta-bullet id #'equal)))
      (prop-match-beginning match))))

(defun peta-journal--nth-row (n)
  "The start of row N, with its date line, or else the end of the buffer."
  (let ((pos (point-min)))
    (dotimes (_ n)
      (setq pos (or (peta-journal--next-row pos) (point-max))))
    pos))

(defun peta-journal--next-row (pos)
  "The start of the row after the one that starts at POS, or nil."
  (let ((end-of-mark (next-single-property-change pos 'peta-row)))
    (and end-of-mark
         (< end-of-mark (point-max))
         (if (get-text-property end-of-mark 'peta-row)
             end-of-mark
           (next-single-property-change end-of-mark 'peta-row)))))

(defun peta-journal--row-index (id &optional rows)
  "The place of the item ID among ROWS, or else among those on screen, or nil."
  (cl-position-if (lambda (row) (equal (plist-get row :id) id))
                  (or rows peta-journal--rows)))

(defun peta-journal--row-of (id)
  "The row of the item ID, or nil."
  (when-let* ((index (peta-journal--row-index id)))
    (aref peta-journal--rows index)))

;;;; Drawing

(defun peta-journal--prefix (level)
  "The indent of the first line of a row at LEVEL."
  (make-string (* 2 (1- level)) ?\s))

(defun peta-journal--body-prefix (level)
  "The indent of the other lines of a row at LEVEL."
  (make-string (* 2 level) ?\s))

(defun peta-journal--task-string (state)
  "The mark of a task in STATE, or an empty string when STATE is nil."
  (if state
      (propertize (concat (cdr (assoc state peta-journal--task-marks))
                          peta-journal--bullet-gap)
                  'font-lock-face (pcase state
                                    ("done" 'peta-journal-done)
                                    ("running" 'peta-journal-running)
                                    (_ 'peta-journal-bullet)))
    ""))

(defun peta-journal--bullet-string (row)
  "The bullet of ROW, with the mark of its task."
  (let* ((kind (plist-get row :bullet))
         (level (plist-get row :level))
         (mark (or (and (plist-get row :collapsed)
                        (cdr (assoc kind peta-journal--collapsed-bullets)))
                   (cdr (assoc kind peta-journal--bullets)))))
    (propertize (concat (propertize (concat mark peta-journal--bullet-gap)
                                    'font-lock-face 'peta-journal-bullet)
                        (peta-journal--task-string (plist-get row :task)))
                'peta-bullet (plist-get row :id)
                'peta-marks (if (plist-get row :task) 4 2)
                'read-only t
                ;; The body starts right behind the bullet, and nothing goes in front of it
                'front-sticky '(read-only)
                'rear-nonsticky '(read-only)
                'line-prefix (peta-journal--prefix level)
                'wrap-prefix (peta-journal--body-prefix level))))

(defun peta-journal--newline-string ()
  "The read-only newline that ends a row."
  (propertize "\n" 'read-only t 'line-spacing peta-journal-item-gap))

(defun peta-journal--header-string (row)
  "The date line above ROW, or an empty string."
  (if-let* ((header (plist-get row :header)))
      (propertize (concat "── " header " ──\n")
                  'peta-header t
                  'font-lock-face 'peta-journal-date-header
                  'read-only t
                  'front-sticky '(read-only))
    ""))

(defun peta-journal--row-string (row)
  "The text of ROW, as it goes into the buffer."
  (let* ((prefix (peta-journal--body-prefix (plist-get row :level)))
         (body (propertize (plist-get row :body) 'line-prefix prefix 'wrap-prefix prefix))
         (text (concat (peta-journal--header-string row)
                       (peta-journal--bullet-string row)
                       body
                       (peta-journal--newline-string))))
    (put-text-property 0 1 'peta-row (plist-get row :id) text)
    text))

(defun peta-journal--keep-typed-body (new ack)
  "Give the edited row in NEW the text on screen, if ACK is behind the typing."
  (when (and peta-journal--current (< ack peta-journal--seq))
    (when-let* ((old (peta-journal--row-of peta-journal--current))
                (index (peta-journal--row-index peta-journal--current new)))
      (setf (aref new index)
            (plist-put (copy-sequence (aref new index)) :body (plist-get old :body))))))

(defun peta-journal--place-of (pos)
  "Where POS is: its item, its place in the body, and the index of the row."
  (when-let* ((start (peta-journal--row-start pos))
              (id (get-text-property start 'peta-bullet)))
    (list id
          (- pos (peta-journal--body-start start))
          (peta-journal--row-index id))))

(defun peta-journal--pos-of-place (place)
  "The position of PLACE, from `peta-journal--place-of'.
When its item has gone, the row now at the same index."
  (seq-let (id offset index) place
    (let ((start (peta-journal--find-row id)))
      (cond
       (start (min (+ (peta-journal--body-start start) offset)
                   (peta-journal--body-end start)))
       ((zerop (length peta-journal--rows)) (point-min))
       (t (peta-journal--row-body-start
           (min index (1- (length peta-journal--rows)))))))))

(defun peta-journal--row-body-start (index)
  "The start of the body of the row at INDEX."
  (peta-journal--body-start
   (peta-journal--find-row (plist-get (aref peta-journal--rows index) :id))))

(defun peta-journal--warn-if-shown-elsewhere ()
  "Say so when another buffer shows the page of the current buffer.
Each buffer shows what the other saves, but only once its own edit has ended.
Two edits of one item overwrite each other."
  (when-let* ((other (peta-journal--other-buffer-of-page (current-buffer))))
    (message "%s also shows this page.  Edits of one item in both buffers overwrite each other"
             (buffer-name other))))

(defun peta-journal--apply-rows (new page-id ack)
  "Show the rows of the vector NEW of PAGE-ID.
The shell has handled the lines up to ACK.  Only the rows that changed are
rewritten, so what does not change stays where it is."
  (peta-journal--keep-typed-body new ack)
  (let* ((new-page (not (equal page-id peta-journal--page-id)))
         (windows (get-buffer-window-list (current-buffer) nil t))
         (window-places (mapcar (lambda (w) (cons w (peta-journal--place-of (window-point w))))
                                windows))
         (place (peta-journal--place-of (point)))
         (old peta-journal--rows)
         (prefix (or (cl-mismatch old new :test #'equal) (length old)))
         ;; From the end, this gives the index in OLD of the last row that differs
         (last-changed (cl-mismatch old new :test #'equal :from-end t
                                    :start1 prefix :start2 prefix))
         (suffix (if last-changed (- (length old) last-changed 1) 0))
         (start (peta-journal--nth-row prefix))
         (end (peta-journal--nth-row (- (length old) suffix)))
         (written-end start)
         (inhibit-read-only t)
         (inhibit-modification-hooks t))
    (save-excursion
      (delete-region start end)
      (goto-char start)
      (cl-loop for index from prefix below (- (length new) suffix)
               do (insert (peta-journal--row-string (aref new index))))
      (setq written-end (point)))
    (setq peta-journal--rows new
          peta-journal--page-id page-id)
    ;; The core has ended the edit of an item that is gone, for one deleted here or by
    ;; another client.  Telling it so again would be about an item it does not know
    (unless (and peta-journal--current (peta-journal--row-index peta-journal--current new))
      (setq peta-journal--current nil))
    (when new-page
      (peta-journal--warn-if-shown-elsewhere)
      (peta-journal--forget-unknown-images))
    (restore-buffer-modified-p nil)
    (cond
     ((zerop (length new)) (goto-char (point-min)))
     ((or new-page (null place))
      (goto-char (peta-journal--row-body-start 0))
      (setq peta-journal--current nil
            peta-journal--scrolled nil))
     (t
      (goto-char (peta-journal--pos-of-place place))
      (pcase-dolist (`(,window . ,window-place) window-places)
        (when window-place
          (set-window-point window (peta-journal--pos-of-place window-place))))))
    (setq peta-journal--last-point (point))
    ;; After the point has its place: the item it is in keeps its text
    (peta-journal--draw-images start written-end)
    (peta-journal--update-images)))

(defun peta-journal--goto-item (id cursor)
  "Put the point in the item ID at CURSOR, or at its end when CURSOR is nil."
  (when-let* ((start (peta-journal--find-row id)))
    (let ((body-start (peta-journal--body-start start))
          (body-end (peta-journal--body-end start)))
      (goto-char (if cursor (min (+ body-start cursor) body-end) body-end)))))

;;;; Keeping the point in a body

(defun peta-journal--row-above (start)
  "The start of the bullet of the row above the one at START, or nil."
  (save-excursion
    (goto-char start)
    (and (zerop (forward-line -1))
         (or (not (get-text-property (point) 'peta-header))
             (zerop (forward-line -1)))
         (peta-journal--row-start (point)))))

(defun peta-journal--step-off-header ()
  "Move the point off a date line, the way it was going."
  (let* ((below (peta-journal--row-start (point)))
         (above (peta-journal--row-above below))
         (backward (and peta-journal--last-point
                        (< (point) peta-journal--last-point))))
    (if (and backward above)
        (goto-char (peta-journal--body-end above))
      (goto-char (peta-journal--body-start below)))))

(defun peta-journal--step-off-bullet (start)
  "Move the point off the bullet at START.
A step back from the start of this body goes on to the row above."
  (let ((body-start (peta-journal--body-start start)))
    (let ((above (and (equal peta-journal--last-point body-start)
                      (peta-journal--row-above start))))
      (goto-char (if above (peta-journal--body-end above) body-start)))))

(defun peta-journal--keep-point-in-body ()
  "Move the point into a body when a command left it on a mark."
  (when-let* ((start (peta-journal--row-start (point))))
    (cond
     ((get-text-property (line-beginning-position) 'peta-header)
      (peta-journal--step-off-header))
     ((< (point) (peta-journal--body-start start))
      (peta-journal--step-off-bullet start))
     ((> (point) (peta-journal--body-end start))
      (goto-char (peta-journal--body-end start))))))

;;;; Images

(defconst peta-journal--image-link-regexp
  "!\\[[^][\n]*\\](\\.\\./assets/\\([A-Za-z0-9._-]+\\)\\(?:#\\([^)\n]*\\)\\)?)"
  "A link to an image on the server.
Group 1 is the name of the file, and group 2 what follows its `#'.  The server
gives an uploaded file such a name, and a name of this form needs no escaping
in a URL.")

;; Global: buffers of the same server show the same files
(defvar peta-journal--images (make-hash-table :test #'equal)
  "What is known of the images by URL: their bytes, `loading' or `missing'.")

(defvar-local peta-journal--text-item nil
  "The id of the item that shows its links as text, where the point was last.")

(defun peta-journal--asset-url (name)
  "The URL of the file NAME in the assets of the server."
  (concat (string-trim-right peta-journal-server "/") "/assets/" name))

(defun peta-journal--image-bytes (url)
  "The bytes of the image at URL, or nil.  The first call starts to fetch them."
  (let ((known (gethash url peta-journal--images)))
    (cond
     ((stringp known) known)
     ((null known)
      (puthash url 'loading peta-journal--images)
      (peta-journal--fetch-image url)
      nil))))

(defun peta-journal--fetch-image (url)
  "Fetch the image at URL without waiting for it.
Not through the shell: a shell handles one line at a time, and an image should
not hold up the typing."
  (condition-case nil
      (url-retrieve url
                    (lambda (status)
                      (let ((bytes (peta-journal--response-bytes status)))
                        (kill-buffer)
                        (peta-journal--image-arrived url bytes)))
                    nil t t)
    (error (puthash url 'missing peta-journal--images))))

(defun peta-journal--response-bytes (status)
  "The body of the HTTP response in this buffer, or nil if STATUS has an error."
  (unless (plist-get status :error)
    (goto-char (point-min))
    (when (re-search-forward "\r?\n\r?\n" nil t)
      (buffer-substring-no-properties (point) (point-max)))))

(defun peta-journal--image-arrived (url bytes)
  "Keep the BYTES of the image at URL, and show the image in every page.
BYTES is nil when the image could not be fetched."
  (puthash url (or bytes 'missing) peta-journal--images)
  (peta-journal--redraw-all-images))

(defun peta-journal--redraw-all-images ()
  "Draw the images of every page again."
  (dolist (page (peta-journal--pages))
    (with-current-buffer page
      (peta-journal--draw-images (point-min) (point-max)))))

(defun peta-journal--forget-unknown-images ()
  "Forget the images without bytes, so they get fetched again.
The server may have been down, and nothing else would try again."
  (maphash (lambda (url known)
             (unless (stringp known)
               (remhash url peta-journal--images)))
           peta-journal--images))

(defun peta-journal--link-side (fragment key)
  "The size that FRAGMENT of a link gives after KEY, or nil.
As in the Web version, only a positive integer counts."
  (when (and fragment
             (string-match (format "\\(?:\\`\\|&\\)%s=\\([0-9]*[1-9][0-9]*\\)\\(?:&\\|\\'\\)" key)
                           fragment))
    (string-to-number (match-string 1 fragment))))

(defun peta-journal--image-room (id)
  "The most an image in the item ID may take, as properties of the image.
An image taller than the window could not be scrolled through line by line."
  (let* ((window (or (get-buffer-window) (selected-window)))
         (level (plist-get (peta-journal--row-of id) :level))
         (indent (length (peta-journal--body-prefix level))))
    (list :max-width (- (window-body-width window t)
                        ;; One more column: an image that fills the line would wrap
                        (* (1+ indent) (frame-char-width)))
          :max-height (- (window-body-height window t) (frame-char-height)))))

(defun peta-journal--link-image (id name fragment)
  "The image of the item ID for a link to the file NAME with FRAGMENT, or nil."
  (when-let* ((bytes (peta-journal--image-bytes (peta-journal--asset-url name))))
    (let ((width (peta-journal--link-side fragment "w"))
          (height (peta-journal--link-side fragment "h")))
      ;; Without a type, this tells it from the bytes, and gives nil if it cannot show them
      (apply #'create-image bytes nil t
             (append (and width (list :width width))
                     (and height (list :height height))
                     (peta-journal--image-room id))))))

(defun peta-journal--in-code-p (pos)
  "Non-nil when the text at POS is code.  A link there is an example, not an image."
  (let ((start (peta-journal--row-start pos)))
    (font-lock-ensure start (peta-journal--body-end start)))
  (memq 'peta-journal-code (ensure-list (get-text-property pos 'face))))

(defun peta-journal--on-window-size-change (_window)
  "Fit the images of the current buffer to the new size of its window."
  (peta-journal--draw-images (point-min) (point-max)))

(defun peta-journal--draw-images (start end)
  "Show the links between START and END as images, but for the item at the point.
START and END are the edges of rows.  An overlay leaves the text alone, so the
body stays as the shell knows it."
  (remove-overlays start end 'peta-image t)
  (when (display-images-p)
    (let ((text-item (peta-journal--id-at (point))))
      (save-excursion
        (goto-char start)
        (while (re-search-forward peta-journal--image-link-regexp end t)
          (let* ((link-start (match-beginning 0))
                 (link-end (match-end 0))
                 (name (match-string-no-properties 1))
                 (fragment (match-string-no-properties 2))
                 (id (peta-journal--id-at link-start)))
            (unless (or (equal id text-item)
                        (peta-journal--in-code-p link-start))
              (when-let* ((image (peta-journal--link-image id name fragment)))
                (let ((overlay (make-overlay link-start link-end)))
                  (overlay-put overlay 'peta-image t)
                  (overlay-put overlay 'display image)
                  ;; The shell may rewrite the row, and nothing should stay behind
                  (overlay-put overlay 'evaporate t))))))))))

(defun peta-journal--draw-images-of (id)
  "Show the links of the item ID as images, when it is there."
  (when-let* ((start (peta-journal--find-row id)))
    (peta-journal--draw-images start (peta-journal--body-end start))))

(defun peta-journal--update-images ()
  "Give the item at the point its text, and the item it has left its images."
  (let ((item (peta-journal--id-at (point)))
        (left peta-journal--text-item))
    (unless (equal item left)
      (setq peta-journal--text-item item)
      (when item
        (peta-journal--draw-images-of item))
      (when left
        (peta-journal--draw-images-of left)))))

;;;; Talking to the shell

(defun peta-journal--send (type &rest fields)
  "Send the shell a line of TYPE with FIELDS."
  (when (process-live-p peta-journal--process)
    (cl-incf peta-journal--seq)
    (let ((line (json-serialize (append (list :seq peta-journal--seq :type type) fields))))
      ;; Since Emacs 30 this is UTF-8 in a unibyte string, and the process encodes what it gets
      (process-send-string peta-journal--process
                           (concat (if (multibyte-string-p line)
                                       line
                                     (decode-coding-string line 'utf-8))
                                   "\n")))))

(defun peta-journal--wait-until (done)
  "Handle the output of the shell until the function DONE says so.
The wait has a limit."
  (let ((deadline (+ (float-time) peta-journal--wait-seconds)))
    (while (and (process-live-p peta-journal--process)
                (not (funcall done))
                (< (float-time) deadline))
      (accept-process-output peta-journal--process 0.05))))

(defun peta-journal--wait ()
  "Wait until the shell has handled every line and the core holds no keys.
Keys pressed in the meantime stay in the input queue of Emacs.  The next one
is handled when the outline it was meant for is on screen."
  (peta-journal--wait-until
   (lambda ()
     (not (or (< peta-journal--ack peta-journal--seq) peta-journal--holding)))))

(defun peta-journal--send-and-wait (type &rest fields)
  "Send the shell a line of TYPE with FIELDS and wait for what it changes."
  (apply #'peta-journal--send type fields)
  (peta-journal--wait))

(defun peta-journal--read-output (output)
  "Handle the complete lines in OUTPUT, and keep the rest for the next piece."
  (let ((lines (split-string (concat peta-journal--output output) "\n")))
    (setq peta-journal--output (car (last lines)))
    (dolist (line (butlast lines))
      (peta-journal--on-line
       (json-parse-string line :object-type 'plist :array-type 'array
                          :null-object nil :false-object nil)))))

(defun peta-journal--pages ()
  "The buffers that show a page, the one used last first."
  (seq-filter (lambda (buffer)
                (eq (buffer-local-value 'major-mode buffer) 'peta-journal-mode))
              (buffer-list)))

(defun peta-journal--page ()
  "The buffer of the page that a command is about, or nil.
This is the current buffer, or else the one in the window a command started
from, or else the one used last.  The minibuffer and timers need this."
  (let ((pages (peta-journal--pages))
        (window (or (minibuffer-selected-window) (selected-window))))
    (seq-find (lambda (buffer) (memq buffer pages))
              (append (list (current-buffer) (window-buffer window)) pages))))

(defun peta-journal--sentinel (process event)
  "Say that the shell PROCESS has stopped, as EVENT tells.
Without a shell, edits would be lost in silence."
  (unless (process-live-p process)
    (let ((page (process-get process 'peta-page)))
      (when (buffer-live-p page)
        (with-current-buffer page
          (setq peta-journal--connection "shell stopped")
          (force-mode-line-update)))
      (display-warning
       'peta-journal
       (format "The shell of %s stopped: %s.  Edits are not saved until it runs again.  \
M-x peta-journal in that buffer starts it and shows the saved text.  See *peta-journal-log*."
               (buffer-name page)
               (string-trim event))
       :error))))

(defun peta-journal--on-line (message)
  "Do what the line MESSAGE from the shell of the current buffer says."
  (pcase (plist-get message :t)
    ("state" (setq peta-journal--ack (plist-get message :ack)
                   peta-journal--holding (plist-get message :holding)))
    ("rows" (peta-journal--apply-rows (plist-get message :rows)
                                      (plist-get message :page_id)
                                      (plist-get message :ack)))
    ("focus" (peta-journal--on-focus (plist-get message :id)
                                     (plist-get message :cursor)
                                     (plist-get message :ack)))
    ("scroll" (peta-journal--on-scroll (plist-get message :id)))
    ("title" (setq peta-journal--title (plist-get message :title))
     (force-mode-line-update))
    ("page_path" (setq peta-journal--page-path (plist-get message :path)))
    ("properties" (peta-journal--on-properties (plist-get message :rows)
                                               (plist-get message :free_keys)
                                               (plist-get message :datetime_keys)))
    ("status" (setq peta-journal--connection (plist-get message :connection))
     (force-mode-line-update))
    ("search" (peta-journal--on-search (plist-get message :results)
                                       (plist-get message :searching)))
    ("recent" (setq peta-journal--recent (append (plist-get message :paths) nil)))
    ("page_names" (setq peta-journal--page-names
                        (cons (plist-get message :query)
                              (append (plist-get message :titles) nil))))
    ("backlinks" (setq peta-journal--backlinks (plist-get message :groups)))
    ("message" (message "%s" (plist-get message :text)))
    ("ask" (peta-journal--on-ask (plist-get message :req)
                                 (plist-get message :message)))))

(defun peta-journal--on-focus (id cursor ack)
  "Put the point in the item ID at CURSOR.
The shell has handled the lines up to ACK.  Nothing happens when lines are
still under way: the point has moved on since."
  (when (>= ack peta-journal--seq)
    (unless (and (null cursor) (equal (peta-journal--id-at (point)) id))
      (peta-journal--goto-item id cursor))
    (when (equal (peta-journal--id-at (point)) id)
      (setq peta-journal--current id
            peta-journal--last-point (point)))
    (peta-journal--update-images)))

(defun peta-journal--on-scroll (id)
  "Show the item ID."
  (when (peta-journal--find-row id)
    (peta-journal--goto-item id 0)
    (setq peta-journal--last-point (point))
    (peta-journal--update-images)
    (dolist (window (get-buffer-window-list (current-buffer) nil t))
      (set-window-point window (point))
      (with-selected-window window (recenter)))))

(defun peta-journal--on-ask (req text)
  "Ask the user the yes-or-no question TEXT, and answer the request REQ.
Not from here but from a timer: a process filter is no place for a prompt."
  (let ((page (current-buffer)))
    (run-at-time
     0 nil
     (lambda ()
       (let ((yes (condition-case nil (yes-or-no-p (concat text " "))
                    (quit nil))))
         (when (buffer-live-p page)
           (with-current-buffer page
             (peta-journal--send "answer" :req req :yes (if yes t :false)))))))))

;;;; Edits

(defun peta-journal--start-edit (id)
  "Tell the core that the item ID is being edited, in place of the one before."
  (unless (equal id peta-journal--current)
    (when peta-journal--current
      (peta-journal--send "edit_finished" :id peta-journal--current))
    (setq peta-journal--current id)
    (peta-journal--send "edit_requested" :id id :cursor (peta-journal--cursor))))

(defun peta-journal--finish-edit ()
  "End the edit of the current item, which saves it."
  (when peta-journal--current
    (peta-journal--send "edit_finished" :id peta-journal--current)
    (setq peta-journal--current nil)))

(defconst peta-journal--commands-that-end-the-edit
  '(peta-journal-save peta-journal-today peta-journal-back
    peta-journal-next-day peta-journal-previous-day peta-journal-open-at-point
    peta-journal-search peta-journal-rename-page)
  "The commands after which the point stays in an item that is not being edited.
A saved edit must stay closed, and a page being left has nothing more to edit.")

(defun peta-journal--post-command ()
  "Keep the point in a body, and the core informed of the item it is in."
  (peta-journal--keep-point-in-body)
  (unless (memq this-command peta-journal--commands-that-end-the-edit)
    (when-let* ((id (peta-journal--id-at (point))))
      (peta-journal--start-edit id)))
  (peta-journal--update-images)
  (setq peta-journal--last-point (point)))

(defun peta-journal--restyle-body (start level)
  "Give the body of the row at START the indent of LEVEL, without copied marks."
  (let ((inhibit-read-only t)
        (inhibit-modification-hooks t)
        (prefix (peta-journal--body-prefix level))
        (body-start (peta-journal--body-start start))
        (body-end (peta-journal--body-end start)))
    (remove-list-of-text-properties body-start body-end
                                    '(peta-bullet peta-marks peta-row peta-header font-lock-face))
    (add-text-properties body-start body-end (list 'line-prefix prefix 'wrap-prefix prefix))))

(defun peta-journal--after-change (beg _end _length)
  "Send the core the body changed at BEG."
  (when-let* ((start (peta-journal--row-start beg))
              (id (get-text-property start 'peta-bullet))
              (row (peta-journal--row-of id)))
    (peta-journal--start-edit id)
    (peta-journal--restyle-body start (plist-get row :level))
    (let ((body (peta-journal--body-at start)))
      (plist-put row :body body)
      (peta-journal--send "body_changed" :id id :body body))
    (restore-buffer-modified-p nil)))

(defun peta-journal--close-typed ()
  "Let the core close the `[[' or the code fence just typed, or go over a `]'.
Only for these characters: waiting on every key would slow typing down."
  (when (memq (char-before) '(?\[ ?\] ?`))
    (peta-journal--send-and-wait "typed"
                                 :id (peta-journal--item)
                                 :cursor (peta-journal--cursor))))

(defun peta-journal--idle ()
  "End the edit while Emacs idles, so the core can take in changes from elsewhere."
  (dolist (page (peta-journal--pages))
    (with-current-buffer page
      (peta-journal--finish-edit))))

(defun peta-journal--item ()
  "The id of the item at the point.  Signals an error when there is none."
  (or (peta-journal--id-at (point)) (user-error "No item here")))

(defun peta-journal--at-body-start-p ()
  "Non-nil when the point is at the start of its body."
  (= (point) (peta-journal--body-start (peta-journal--row-start (point)))))

(defun peta-journal--at-body-end-p ()
  "Non-nil when the point is at the end of its body."
  (= (point) (peta-journal--body-end (peta-journal--row-start (point)))))

;;;; Commands on items

(defun peta-journal-enter ()
  "Split the item at the point, or add an item below it."
  (interactive)
  (peta-journal--send-and-wait "enter" :id (peta-journal--item) :cursor (peta-journal--cursor)))

(defun peta-journal-newline ()
  "Add a line to the item at the point, without splitting it."
  (interactive)
  (insert "\n"))

(defun peta-journal-backspace (&optional n)
  "Delete N characters back.  At the start of a body, join it to the item above."
  (interactive "p")
  (if (and (not (use-region-p)) (peta-journal--at-body-start-p))
      (peta-journal--send-and-wait "backspace_at_start" :id (peta-journal--item))
    (delete-char (- (or n 1)))))

(defun peta-journal-delete (&optional n)
  "Delete N characters ahead.  At the end of a body, join the item below to it."
  (interactive "p")
  (if (and (not (use-region-p)) (peta-journal--at-body-end-p))
      (peta-journal--send-and-wait "delete_at_end" :id (peta-journal--item))
    (delete-char (or n 1))))

(defun peta-journal-kill-line ()
  "Kill the rest of the line.  At the end of a body, join the item below to it."
  (interactive)
  (if (peta-journal--at-body-end-p)
      (peta-journal--send-and-wait "delete_at_end" :id (peta-journal--item))
    (kill-line)))

(defun peta-journal-beginning-of-line ()
  "Move to the start of the line, which on a first line is behind the bullet."
  (interactive)
  (beginning-of-line)
  (when-let* ((start (peta-journal--row-start (point))))
    (when (= start (point))
      (goto-char (peta-journal--body-start start)))))

(defun peta-journal-indent ()
  "Indent the item at the point, with its children."
  (interactive)
  (peta-journal--send-and-wait "indent" :id (peta-journal--item)))

(defun peta-journal-outdent ()
  "Outdent the item at the point, with its children."
  (interactive)
  (peta-journal--send-and-wait "outdent" :id (peta-journal--item)))

(defun peta-journal-move-up ()
  "Move the item at the point up, with its children."
  (interactive)
  (peta-journal--send-and-wait "move_up" :id (peta-journal--item)))

(defun peta-journal-move-down ()
  "Move the item at the point down, with its children."
  (interactive)
  (peta-journal--send-and-wait "move_down" :id (peta-journal--item)))

(defun peta-journal-task-cycle ()
  "Make the item at the point a task, or take its task to the next state."
  (interactive)
  (peta-journal--send-and-wait "task_cycle" :id (peta-journal--item)))

(defun peta-journal-collapse-toggle ()
  "Hide or show the children of the item at the point."
  (interactive)
  (peta-journal--send-and-wait "collapse_toggle" :id (peta-journal--item)))

(defun peta-journal-delete-item ()
  "Delete the item at the point, with its children.
The edit of the item goes on until the rows without it arrive: the user or the
server may still say no."
  (interactive)
  (peta-journal--send-and-wait "delete_item" :id (peta-journal--item)))

(defun peta-journal--clipboard-image ()
  "The image in the clipboard of the OS as the bytes of a PNG file, or nil.
Nil as well when the clipboard has text: then the text is what gets pasted.
Why the list of types is read first: asking for a type that is not there is an
error on some systems."
  (let ((types (append (gui-get-selection 'CLIPBOARD 'TARGETS) nil)))
    (and (memq 'image/png types)
         (not (memq 'UTF8_STRING types))
         (not (memq 'STRING types))
         (gui-get-selection 'CLIPBOARD 'image/png))))

(defun peta-journal--paste-image (png)
  "Upload the image with the bytes PNG and put a link to it at the point."
  (let* ((folder (make-temp-file "peta-journal" t))
         ;; The server names the upload after the file.  A browser gives a pasted image this name
         (file (expand-file-name "image.png" folder)))
    (unwind-protect
        (progn
          (let ((coding-system-for-write 'no-conversion))
            (write-region png nil file nil 'silent))
          (peta-journal-insert-image file))
      (delete-directory folder t))))

(defun peta-journal-yank ()
  "Paste the latest kill.  The core splits it into items when it has several.
When the clipboard of the OS has an image and no text, upload the image and
put a link to it at the point."
  (interactive)
  (if-let* ((image (peta-journal--clipboard-image)))
      (peta-journal--paste-image image)
    (peta-journal--send-and-wait "paste"
                                 :id (peta-journal--item)
                                 :text (substring-no-properties (current-kill 0))
                                 :cursor (peta-journal--cursor))))

(defconst peta-journal--image-file-regexp
  "\\.\\(?:bmp\\|gif\\|jpe?g\\|png\\|svg\\|webp\\)\\'"
  "The names of the files that can be inserted as an image.")

(defun peta-journal--image-file-p (file)
  "Non-nil when the name of FILE is that of an image."
  (let ((case-fold-search t))
    (string-match-p peta-journal--image-file-regexp file)))

(defun peta-journal--read-image-file ()
  "Ask for an image file.  The choices include folders, to go into them."
  (read-file-name "Insert image: " nil nil t nil
                  (lambda (file)
                    (or (file-directory-p file) (peta-journal--image-file-p file)))))

(defun peta-journal-insert-image (file)
  "Upload the image FILE and put a link to it at the point."
  (interactive
   (progn
     ;; Before asking: a file chosen for no item would be wasted
     (peta-journal--item)
     (list (peta-journal--read-image-file))))
  (unless (peta-journal--image-file-p file)
    (user-error "Not an image file: %s" file))
  (peta-journal--send-and-wait "insert_image"
                               :id (peta-journal--item)
                               :path (expand-file-name file)
                               :cursor (peta-journal--cursor)))

(defun peta-journal-undo ()
  "Undo the last change of the outline."
  (interactive)
  (peta-journal--send-and-wait "undo"))

(defun peta-journal-redo ()
  "Redo the change undone last."
  (interactive)
  (peta-journal--send-and-wait "redo"))

(defun peta-journal-save ()
  "Save the item at the point now.  It is saved by itself after a pause as well."
  (interactive)
  (peta-journal--finish-edit)
  (peta-journal--wait))

;;;; Commands that go to other pages

(defun peta-journal--go (type &rest fields)
  "End the edit, and send the line of TYPE with FIELDS that changes the page."
  (with-current-buffer (peta-journal--page)
    (peta-journal--finish-edit)
    (apply #'peta-journal--send type fields)))

(defun peta-journal-open-at-point ()
  "Open the page of the wiki link or the tag at the point."
  (interactive)
  (peta-journal--go "open_link_at" :id (peta-journal--item) :cursor (peta-journal--cursor)))

(defun peta-journal-today ()
  "Open today's journal."
  (interactive)
  (peta-journal--ensure-started)
  (peta-journal--go "go" :url "/today"))

(defun peta-journal-rename-page (path)
  "Give the page on screen the new PATH.  A `/' in PATH puts the page in a folder."
  (interactive
   (let* ((page (peta-journal--page))
          (current (and page (buffer-local-value 'peta-journal--page-path page))))
     (unless current
       (user-error "Only a doc page can be renamed, not a journal, an index or an atlas"))
     (list (read-string "Rename page to: " current))))
  (peta-journal--go "rename_page" :path path))

(defun peta-journal-back ()
  "Go back to the page shown before."
  (interactive)
  (peta-journal--go "back"))

(defun peta-journal-next-day ()
  "Open the next journal."
  (interactive)
  (peta-journal--go "journal_nav" :forward t))

(defun peta-journal-previous-day ()
  "Open the previous journal."
  (interactive)
  (peta-journal--go "journal_nav" :forward :false))

;;;; The calendar

(defvar peta-journal--calendar-page nil
  "The buffer of the page that the calendar picks a date for.")

(defun peta-journal--calendar-open (unit)
  "Close the calendar, and open the page of UNIT of the date at the point."
  (let* ((date (or (calendar-cursor-to-date)
                   (user-error "The point is not on a date")))
         (iso (format "%04d-%02d-%02d" (nth 2 date) (nth 0 date) (nth 1 date)))
         (page peta-journal--calendar-page))
    (calendar-exit)
    (when-let* ((window (get-buffer-window page)))
      (select-window window))
    (with-current-buffer page
      (peta-journal--go "go_to_date" :date iso :unit unit))))

(defun peta-journal-calendar-open-day ()
  "Open the journal of the day at the point of the calendar."
  (interactive)
  (peta-journal--calendar-open "day"))

(defun peta-journal-calendar-open-week ()
  "Open the page of the week of the day at the point of the calendar.
The server sets how the weeks are numbered."
  (interactive)
  (peta-journal--calendar-open "week"))

(defun peta-journal-calendar-open-month ()
  "Open the page of the month of the day at the point of the calendar."
  (interactive)
  (peta-journal--calendar-open "month"))

(defun peta-journal-calendar-open-year ()
  "Open the page of the year of the day at the point of the calendar."
  (interactive)
  (peta-journal--calendar-open "year"))

(defvar-keymap peta-journal-calendar-mode-map
  :doc "Keys of the calendar that open a page of PetaJournal."
  "RET" #'peta-journal-calendar-open-day
  "w" #'peta-journal-calendar-open-week
  "m" #'peta-journal-calendar-open-month
  "y" #'peta-journal-calendar-open-year)

(define-minor-mode peta-journal-calendar-mode
  "Open a page of PetaJournal for the date at the point of the calendar.
`peta-journal-calendar' turns this on.  A calendar opened by itself does not
have it, as it starts its mode again.

\\{peta-journal-calendar-mode-map}"
  :lighter " Peta")

(defun peta-journal-calendar ()
  "Pick a date in the calendar of Emacs, and open its journal.
The calendar opens with its usual keys, and with these:

\\{peta-journal-calendar-mode-map}
Closing the calendar stays on the page."
  (interactive)
  ;; Without a page, as from another buffer by M-x, this starts with today's journal
  (let ((page (or (peta-journal--page) (peta-journal--ensure-started))))
    (calendar)
    (setq peta-journal--calendar-page page)
    (peta-journal-calendar-mode 1)
    ;; The calendar itself shows none of these keys
    (message "%s"
             (substitute-command-keys
              "\\<peta-journal-calendar-mode-map>\
\\[peta-journal-calendar-open-day]: day  \
\\[peta-journal-calendar-open-week]: week  \
\\[peta-journal-calendar-open-month]: month  \
\\[peta-journal-calendar-open-year]: year  \
\\<calendar-mode-map>\\[calendar-exit]: close"))))

;;;; Lists of results

(defvar-keymap peta-journal-list-mode-map
  :parent special-mode-map
  "RET" #'peta-journal-list-open
  "n" #'next-line
  "p" #'previous-line)

(define-derived-mode peta-journal-list-mode special-mode "PetaJournal List"
  "A list of places in PetaJournal.  RET opens the one at the point.")

(defun peta-journal-list-open ()
  "Open the entry at the point."
  (interactive)
  (let ((action (get-text-property (point) 'peta-action)))
    (unless action
      (user-error "No entry here"))
    (quit-window)
    (funcall action)))

(defun peta-journal--strip-tags (html)
  "HTML without its tags, as the search snippets mark their hits."
  (replace-regexp-in-string "<[^>]+>" "" html))

(defun peta-journal--search-line (entry)
  "The text of the search result ENTRY."
  (let ((title (plist-get entry :title))
        (snippet (peta-journal--strip-tags (plist-get entry :snippet))))
    (pcase (plist-get entry :kind)
      ("create" (format "[新規] %s" title))
      ("item" (format "%s: %s" title (string-replace "\n" " " snippet)))
      (_ title))))

(defconst peta-journal--search-buffer "*PetaJournal Search*"
  "The name of the list of search results.")

(defvar peta-journal--searching nil
  "Non-nil while the results are not yet those of the latest query.")

(defvar peta-journal--search-history nil "The queries typed so far.")

(defvar-local peta-journal--search-entries []
  "The search results on screen.")
(defvar-local peta-journal--search-selected nil
  "The line of the selected result, or nil.")
(defvar-local peta-journal--search-marks nil
  "The overlays that mark the selected result.")

(defun peta-journal--search-show-selection ()
  "Mark the selected result, and scroll the list to it."
  (mapc #'delete-overlay peta-journal--search-marks)
  (setq peta-journal--search-marks nil)
  (when peta-journal--search-selected
    (goto-char (point-min))
    (forward-line peta-journal--search-selected)
    (let ((line (make-overlay (point) (line-beginning-position 2)))
          (arrow (make-overlay (point) (+ (point) 2))))
      (overlay-put line 'face 'highlight)
      (overlay-put arrow 'display "> ")
      (setq peta-journal--search-marks (list line arrow)))
    (when-let* ((window (get-buffer-window (current-buffer) t)))
      (set-window-point window (point)))))

(defun peta-journal--search-render (results)
  "Show RESULTS.  The selection stays when they are on screen already."
  (unless (equal results peta-journal--search-entries)
    (let ((inhibit-read-only t))
      (erase-buffer)
      (seq-doseq (entry results)
        (insert "  " (peta-journal--search-line entry) "\n")))
    (setq peta-journal--search-entries results
          peta-journal--search-selected (and (> (length results) 0) 0))
    (peta-journal--search-show-selection)))

(defun peta-journal--on-search (results searching)
  "Show RESULTS in the list, when it is open.
SEARCHING says that the results of a newer query are coming."
  (setq peta-journal--searching searching)
  (when-let* ((buffer (get-buffer peta-journal--search-buffer)))
    (with-current-buffer buffer
      (peta-journal--search-render results))))

(defun peta-journal--search-open ()
  "Open the empty list of search results below, without selecting it."
  (with-current-buffer (get-buffer-create peta-journal--search-buffer)
    (special-mode)
    (display-buffer (current-buffer)
                    '((display-buffer-at-bottom) (window-height . 0.4)))))

(defun peta-journal--search-close ()
  "Close the list of search results."
  (when-let* ((buffer (get-buffer peta-journal--search-buffer)))
    (when-let* ((window (get-buffer-window buffer t)))
      (quit-restore-window window 'bury))
    (kill-buffer buffer)))

(defun peta-journal--search-entry ()
  "The selected result, or nil."
  (when-let* ((buffer (get-buffer peta-journal--search-buffer)))
    (with-current-buffer buffer
      (when peta-journal--search-selected
        (aref peta-journal--search-entries peta-journal--search-selected)))))
(defun peta-journal--search-move (step)
  "Move the selection by STEP results, going round at the ends.
This waits for the results of the query typed, as their arrival resets the
selection.  The keys pressed in the meantime stay in the input queue."
  (peta-journal--search-settled-p)
  (with-current-buffer peta-journal--search-buffer
    (when peta-journal--search-selected
      (setq peta-journal--search-selected
            (mod (+ peta-journal--search-selected step)
                 (length peta-journal--search-entries)))
      (peta-journal--search-show-selection))))

(defun peta-journal-search-next ()
  "Select the next search result."
  (interactive)
  (peta-journal--search-move 1))

(defun peta-journal-search-previous ()
  "Select the previous search result."
  (interactive)
  (peta-journal--search-move -1))

(defun peta-journal--search-settled-p ()
  "Wait for the results of the query typed.  Non-nil when they have arrived.
The core waits for a pause in the typing before it searches.  Without this
wait, a quick RET would open a result of an older query."
  (with-current-buffer (or (peta-journal--page) (current-buffer))
    (let ((settled (lambda ()
                     (and (>= peta-journal--ack peta-journal--seq)
                          (not peta-journal--searching)))))
      (peta-journal--wait-until settled)
      (funcall settled))))

(defun peta-journal-search-exit ()
  "Open the selected search result."
  (interactive)
  (cond
   ((not (peta-journal--search-settled-p))
    (minibuffer-message "The search does not answer"))
   ((peta-journal--search-entry) (exit-minibuffer))
   (t (minibuffer-message "No result to open"))))

(defun peta-journal-search-recent ()
  "List the recent pages as the results, when nothing is typed yet."
  (interactive)
  (when (string-empty-p (minibuffer-contents-no-properties))
    (with-current-buffer peta-journal--search-buffer
      (peta-journal--search-render
       (vconcat (mapcar (lambda (path) (list :kind "recent" :title path :snippet "" :path path))
                        peta-journal--recent))))))

(defun peta-journal--search-input-changed (_beg _end _length)
  "Search for what the minibuffer holds now."
  (let ((query (minibuffer-contents-no-properties)))
    (with-current-buffer (peta-journal--page)
      (peta-journal--send "search" :query query))))

(defvar-keymap peta-journal-search-minibuffer-map
  :parent minibuffer-local-map
  "RET" #'peta-journal-search-exit
  "C-j" #'peta-journal-search-exit
  "TAB" #'peta-journal-search-recent
  "C-n" #'peta-journal-search-next
  "C-p" #'peta-journal-search-previous
  "<down>" #'peta-journal-search-next
  "<up>" #'peta-journal-search-previous)

;;;###autoload
(defun peta-journal-search ()
  "Search as you type, and open a page, a new page or an item from the results.
TAB before typing lists the recent pages.  C-n and C-p select a result, RET
opens it, and C-g closes the search."
  (interactive)
  (let ((page (peta-journal--ensure-started))
        (chosen nil))
    (peta-journal--search-open)
    (unwind-protect
        (progn
          (minibuffer-with-setup-hook
              (lambda ()
                (add-hook 'after-change-functions
                          #'peta-journal--search-input-changed nil t))
            (read-from-minibuffer "Search: " nil peta-journal-search-minibuffer-map
                                  nil 'peta-journal--search-history))
          (setq chosen (peta-journal--search-entry)))
      (peta-journal--search-close)
      (unless chosen
        (with-current-buffer page
          (peta-journal--send "search" :query ""))))
    (with-current-buffer page
      ;; A recent page is not a result of the core, so it has a path and no number
      (if-let* ((path (plist-get chosen :path)))
          (peta-journal--go "go_to_page" :path path)
        (peta-journal--go "search_result" :position (plist-get chosen :position))))
    (if-let* ((window (get-buffer-window page)))
        (select-window window)
      (pop-to-buffer-same-window page))))

(defun peta-journal-backlinks ()
  "List the pages that link to the page on screen."
  (interactive)
  (let ((page (current-buffer))
        (groups peta-journal--backlinks))
    (with-current-buffer (get-buffer-create "*PetaJournal Backlinks*")
      (peta-journal-list-mode)
      (let ((inhibit-read-only t))
        (erase-buffer)
        (insert "Backlinks\n\n")
        (seq-doseq (group groups)
          (peta-journal--insert-backlink page (plist-get group :title) (plist-get group :url))
          (seq-doseq (item (plist-get group :items))
            (peta-journal--insert-backlink page
                                           (concat "  " (plist-get item :text))
                                           (plist-get item :url))))
        (goto-char (point-min))
        (forward-line 2))))
  (pop-to-buffer "*PetaJournal Backlinks*"))

;;;; Page properties

(defconst peta-journal--properties-buffer "*PetaJournal Properties*"
  "The name of the buffer that lists the properties.")

(defvar-local peta-journal--list-page nil
  "The buffer of the page that the list of properties shows.")

(defconst peta-journal--local-datetime-format "%Y-%m-%d %H:%M"
  "How a date and a time look in Emacs: in local time, to the minute.")

(defun peta-journal--local-datetime (value)
  "VALUE, a date and a time from the server, in local time.
A value that is not one stays as it is: it may have been written elsewhere."
  (let ((time (ignore-errors (parse-iso8601-time-string value))))
    (if time
        (format-time-string peta-journal--local-datetime-format time)
      value)))

(defun peta-journal--server-datetime (local)
  "The date and time LOCAL as the server stores it, or nil when it is not one."
  (when (string-match "\\`\\([0-9]\\{4\\}\\)-\\([0-9]\\{2\\}\\)-\\([0-9]\\{2\\}\\) \\([0-9]\\{2\\}\\):\\([0-9]\\{2\\}\\)\\'"
                      local)
    (let* ((part (lambda (group) (string-to-number (match-string group local))))
           (time (encode-time (list 0 (funcall part 5) (funcall part 4)
                                    (funcall part 3) (funcall part 2) (funcall part 1)
                                    nil -1 nil))))
      ;; `encode-time' carries February 30 over to March: only a real date comes back the same
      (when (equal (format-time-string peta-journal--local-datetime-format time) local)
        (format-time-string "%Y-%m-%dT%H:%M:%SZ" time t)))))

(defun peta-journal--read-datetime (key stored)
  "Ask for the date and time of KEY, whose value is STORED or nil on a new key.
Return it as the server stores it, or an empty string when the answer is empty."
  (let* ((start (if stored
                    (peta-journal--local-datetime stored)
                  (format-time-string peta-journal--local-datetime-format (current-time))))
         (typed (read-string (format "Value of %s (YYYY-MM-DD HH:MM): " key) start)))
    (cond ((string-empty-p typed) typed)
          ;; The list shows minutes: reading it back would lose the seconds
          ((and stored (equal typed start)) stored)
          ((peta-journal--server-datetime typed))
          (t (user-error "Not a date and time: %s.  Write it as YYYY-MM-DD HH:MM" typed)))))

(defvar-keymap peta-journal-properties-mode-map
  "s" #'peta-journal-set-property
  "d" #'peta-journal-delete-property)

(define-derived-mode peta-journal-properties-mode peta-journal-list-mode "PetaJournal Properties"
  "The properties of a page.
s sets the one at the point, d removes it, and q closes the list.")

(defun peta-journal--draw-properties (page)
  "Fill the list of properties with those of the buffer PAGE.
The point stays on its line, so that the next key works on the same property."
  (let ((rows (buffer-local-value 'peta-journal--properties page))
        (datetime-keys (buffer-local-value 'peta-journal--datetime-keys page)))
    (with-current-buffer (get-buffer-create peta-journal--properties-buffer)
      (peta-journal-properties-mode)
      (setq peta-journal--list-page page)
      (let* ((inhibit-read-only t)
             (window (get-buffer-window (current-buffer)))
             (line (line-number-at-pos (if window (window-point window) (point))))
             (width (seq-max (cons 0 (mapcar (lambda (row) (string-width (plist-get row :key)))
                                             rows)))))
        (erase-buffer)
        (insert "Properties\n\n")
        (seq-doseq (row rows)
          (let* ((key (plist-get row :key))
                 (value (plist-get row :value)))
            (insert (propertize (format "%s%s  %s\n"
                                        key
                                        (make-string (- width (string-width key)) ?\s)
                                        (if (member key datetime-keys)
                                            (peta-journal--local-datetime value)
                                          value))
                                'peta-property-key key))))
        (goto-char (point-min))
        (forward-line (1- line))
        (when window
          (set-window-point window (point)))))))

(defun peta-journal--on-properties (rows free-keys datetime-keys)
  "Keep the property ROWS, FREE-KEYS and DATETIME-KEYS.
Draw the list when it shows this page."
  (setq peta-journal--properties rows
        peta-journal--free-keys (append free-keys nil)
        peta-journal--datetime-keys (append datetime-keys nil))
  (when-let* ((list-buffer (get-buffer peta-journal--properties-buffer)))
    (when (eq (buffer-local-value 'peta-journal--list-page list-buffer) (current-buffer))
      (peta-journal--draw-properties (current-buffer)))))

(defun peta-journal--properties-page ()
  "The buffer of the page that a property command is about.
Signals an error on a journal, which cannot have properties."
  (let ((page (peta-journal--page)))
    (unless (and page (buffer-local-value 'peta-journal--properties page))
      (user-error "Only a doc page has properties, not a journal"))
    page))

(defun peta-journal--show-properties (page)
  "Show the list of properties of the buffer PAGE, and keep the point where it is."
  (peta-journal--draw-properties page)
  (display-buffer peta-journal--properties-buffer))

(defun peta-journal-properties ()
  "Show the properties of the page in another window and go there.
When the window is open already, close it."
  (interactive)
  (let ((page (peta-journal--properties-page))
        (window (get-buffer-window peta-journal--properties-buffer)))
    (if window
        (quit-window nil window)
      (peta-journal--show-properties page)
      (select-window (get-buffer-window peta-journal--properties-buffer))
      (goto-char (point-min))
      (forward-line 2))))

(defun peta-journal--property-keys (page field)
  "The keys of the properties of the buffer PAGE whose FIELD is true."
  (mapcar (lambda (row) (plist-get row :key))
          (seq-filter (lambda (row) (plist-get row field))
                      (buffer-local-value 'peta-journal--properties page))))

(defun peta-journal--read-property-key (prompt keys)
  "Ask with PROMPT for one of KEYS.  Signals an error when the answer is empty.
In the list of properties, the key on the line of the point is the default."
  (let* ((here (get-text-property (line-beginning-position) 'peta-property-key))
         (key (completing-read prompt keys nil t nil nil (car (member here keys)))))
    (when (string-empty-p key)
      (user-error "No property was chosen"))
    key))

(defun peta-journal--read-property-to-set ()
  "Ask for a key and its value.  Return them as a list."
  (let* ((page (peta-journal--properties-page))
         (key (peta-journal--read-property-key
               "Set property: "
               (append (peta-journal--property-keys page :editable)
                       (buffer-local-value 'peta-journal--free-keys page))))
         (row (seq-find (lambda (row) (equal (plist-get row :key) key))
                        (buffer-local-value 'peta-journal--properties page)))
         (value (if (member key (buffer-local-value 'peta-journal--datetime-keys page))
                    (peta-journal--read-datetime key (plist-get row :value))
                  (read-string (format "Value of %s: " key) (plist-get row :value)))))
    (when (string-empty-p value)
      (user-error "The value is empty.  To remove a property, use `peta-journal-delete-property'"))
    (list key value)))

(defun peta-journal--change-property (type &rest fields)
  "Send the line of TYPE with FIELDS for the page, and show its list of properties."
  (let ((page (peta-journal--properties-page)))
    (with-current-buffer page
      (apply #'peta-journal--send type fields))
    (peta-journal--show-properties page)))

(defun peta-journal-set-property (key value)
  "Set the property KEY of the page to VALUE, and show the list of properties."
  (interactive (peta-journal--read-property-to-set))
  (peta-journal--change-property "set_property" :key key :value value))

(defun peta-journal-delete-property (key)
  "Remove the property KEY from the page, and show the list of properties."
  (interactive
   (let ((keys (peta-journal--property-keys (peta-journal--properties-page) :removable)))
     (unless keys
       (user-error "This page has no property to remove"))
     (list (peta-journal--read-property-key "Remove property: " keys))))
  (peta-journal--change-property "delete_property" :key key))

(defun peta-journal--insert-backlink (page text url)
  "Insert the line TEXT of the list of backlinks, which opens URL in PAGE."
  (insert (propertize (concat text "\n")
                      'peta-action (lambda ()
                                     (with-current-buffer page
                                       (peta-journal--go "go" :url url))))))

;;;; Completing a wiki link

(defun peta-journal--wiki-link-bounds ()
  "The start and the end of the page name of the wiki link at the point, or nil.
The name stands between `[[' and `]]'.  Without a `]]', it ends at the point."
  (when-let* ((row (peta-journal--row-start (point))))
    (let ((body-start (peta-journal--body-start row))
          (body-end (peta-journal--body-end row))
          (point (point)))
      (save-excursion
        (skip-chars-backward "^][\n" body-start)
        (when (and (<= (+ body-start 2) (point))
                   (string= (buffer-substring-no-properties (- (point) 2) (point)) "[["))
          (let ((start (point)))
            (goto-char point)
            (skip-chars-forward "^][\n" body-end)
            (list start (if (looking-at-p "\\]\\]") (point) point))))))))

(defun peta-journal--find-page-names (id typed)
  "The page names the server finds for TYPED in a wiki link of the item ID.
Without TYPED there is nothing to search for, so these are the recent pages."
  (if (string-empty-p typed)
      peta-journal--recent
    (setq peta-journal--page-names nil)
    (peta-journal--send "page_names" :id id :query typed)
    (peta-journal--wait-until (lambda () (equal (car peta-journal--page-names) typed)))
    (cdr peta-journal--page-names)))

(defun peta-journal--page-name-table (id)
  "A completion table of the page names for a wiki link of the item ID.
It asks the server when the names are needed, and once for each text: Emacs
reads a table several times in one completion."
  (let ((page (current-buffer))
        (found nil))
    (lambda (typed predicate action)
      (cond
       ((eq action 'metadata)
        ;; The server sorts by how well a name matches, so its order is kept
        '(metadata (category . peta-journal-page)
                   (display-sort-function . identity)
                   (cycle-sort-function . identity)))
       ((eq (car-safe action) 'boundaries) nil)
       (t
        (let ((names (cond
                      ((assoc typed found) (cdr (assoc typed found)))
                      ;; A name that was put in is read once more, to see that it is complete
                      ((seq-some (lambda (asked) (member typed (cdr asked))) found)
                       (list typed))
                      (t (let ((names (with-current-buffer page
                                        (peta-journal--find-page-names id typed))))
                           (push (cons typed names) found)
                           names)))))
          (if (eq action t)
              ;; All of them: the server has chosen, and not only by the start of a name
              (all-completions "" names predicate)
            (complete-with-action action names typed predicate))))))))

(defun peta-journal--try-found-names (typed table predicate point)
  "Complete TYPED to the only name TABLE found, and leave it when there are more.
PREDICATE and POINT are as in any completion style."
  (let ((names (all-completions typed table predicate)))
    (cond ((null names) nil)
          ((equal names (list typed)) t)
          ((null (cdr names)) (cons (car names) (length (car names))))
          (t (cons typed point)))))

(defun peta-journal--all-found-names (typed table predicate _point)
  "Every name TABLE found for TYPED that passes PREDICATE."
  (all-completions typed table predicate))

;; The usual styles keep only the names that start with the typed text.  The server also
;; finds a name by a folder in its path or by a word inside it.
(add-to-list 'completion-styles-alist
             '(peta-journal-found
               peta-journal--try-found-names peta-journal--all-found-names
               "The page names of PetaJournal as the server found them."))
(add-to-list 'completion-category-defaults '(peta-journal-page (styles peta-journal-found)))

(defun peta-journal--complete-wiki-link ()
  "Complete the page name of the wiki link at the point.
This is for `completion-at-point-functions'."
  (when-let* ((bounds (peta-journal--wiki-link-bounds)))
    (list (car bounds) (cadr bounds) (peta-journal--page-name-table (peta-journal--item)))))

;;;; The mode

(defvar-keymap peta-journal-mode-map
  "RET" #'peta-journal-enter
  "C-j" #'peta-journal-newline
  "S-<return>" #'peta-journal-newline
  "TAB" #'peta-journal-indent
  "<backtab>" #'peta-journal-outdent
  ;; Not M-<arrow>: those keys keep their Emacs meanings, such as moving by a word.
  "M-S-<right>" #'peta-journal-indent
  "M-S-<left>" #'peta-journal-outdent
  "M-S-<up>" #'peta-journal-move-up
  "M-S-<down>" #'peta-journal-move-down
  ;; C-c C-c is left free: in Emacs it means "confirm" or "run".
  "C-c C-t" #'peta-journal-task-cycle
  "C-c C-f" #'peta-journal-collapse-toggle
  "C-c C-d" #'peta-journal-delete-item
  "C-c C-o" #'peta-journal-open-at-point
  "C-c C-j" #'peta-journal-today
  "C-c C-s" #'peta-journal-search
  "C-c C-b" #'peta-journal-back
  "C-c C-n" #'peta-journal-next-day
  "C-c C-p" #'peta-journal-previous-day
  ;; Page-level commands, used less often while writing, go under C-c C-x as in Org.
  "C-c C-x b" #'peta-journal-backlinks
  "C-c C-x r" #'peta-journal-rename-page
  "C-c C-x p" #'peta-journal-properties
  "C-c C-x n" #'peta-journal-new-buffer
  "C-c C-x i" #'peta-journal-insert-image
  "C-c C-x c" #'peta-journal-calendar
  ;; In `text-mode' this key completes a word from the dictionary in some versions of Emacs
  "C-M-i" #'completion-at-point
  "<remap> <delete-backward-char>" #'peta-journal-backspace
  "<remap> <delete-char>" #'peta-journal-delete
  "<remap> <delete-forward-char>" #'peta-journal-delete
  "<remap> <kill-line>" #'peta-journal-kill-line
  "<remap> <move-beginning-of-line>" #'peta-journal-beginning-of-line
  "<remap> <yank>" #'peta-journal-yank
  "<remap> <undo>" #'peta-journal-undo
  "<remap> <undo-redo>" #'peta-journal-redo
  "<remap> <save-buffer>" #'peta-journal-save)

(defun peta-journal--header-line ()
  "The line above the page: its title, and the connection when it is not up."
  (concat " " peta-journal--title
          (unless (member peta-journal--connection '("" "connected"))
            (format "  [%s]" peta-journal--connection))))

(defvar font-lock-beg)
(defvar font-lock-end)

(defun peta-journal--extend-font-lock-region ()
  "Grow the region font-lock is about to color to whole bodies.
A code block or a quote spans lines, and a line alone does not say it is in one."
  (let* ((first-row (peta-journal--row-start font-lock-beg))
         (last-row (peta-journal--row-start (max font-lock-beg (1- font-lock-end))))
         (beg (if first-row (min first-row font-lock-beg) font-lock-beg))
         (end (if last-row
                  (max font-lock-end (min (1+ (peta-journal--body-end last-row)) (point-max)))
                font-lock-end)))
    (unless (and (= beg font-lock-beg) (= end font-lock-end))
      (setq font-lock-beg beg
            font-lock-end end)
      t)))

(define-derived-mode peta-journal-mode text-mode "PetaJournal"
  "One page of PetaJournal in one buffer.
Type in a body as in any buffer.  The outline changes through these keys:
\\{peta-journal-mode-map}"
  ;; The core keeps the undo history: this one would not know about the other items
  (setq buffer-undo-list t)
  (setq-local word-wrap t)
  ;; Japanese has no spaces: without this, a long sentence counts as one word and moves to a new line
  (setq-local word-wrap-by-category t)
  (setq-local nobreak-char-display nil)
  ;; Long items wrap all the time here: a mark on each wrapped line would only add noise
  (setq-local fringe-indicator-alist
              (cons '(continuation) fringe-indicator-alist))
  (setq-local font-lock-defaults '(peta-journal--font-lock-keywords t))
  (setq-local font-lock-extend-region-functions
              (append font-lock-extend-region-functions
                      '(peta-journal--extend-font-lock-region)))
  (setq-local header-line-format '(:eval (peta-journal--header-line)))
  (add-hook 'completion-at-point-functions #'peta-journal--complete-wiki-link nil t)
  (add-hook 'after-change-functions #'peta-journal--after-change nil t)
  (add-hook 'post-self-insert-hook #'peta-journal--close-typed nil t)
  (add-hook 'post-command-hook #'peta-journal--post-command nil t)
  (add-hook 'window-scroll-functions #'peta-journal--on-window-scroll nil t)
  (add-hook 'window-size-change-functions #'peta-journal--on-window-size-change nil t)
  (add-hook 'kill-buffer-hook #'peta-journal-stop nil t))

(defun peta-journal--on-window-scroll (_window _start)
  "Report the scroll position soon.  Not from here: this runs during redisplay."
  (unless peta-journal--scroll-timer
    (setq peta-journal--scroll-timer
          (run-with-idle-timer 0.2 nil #'peta-journal--report-scroll (current-buffer)))))

(defun peta-journal--report-scroll (page)
  "Tell the core which rows of PAGE are on screen.
A journal then loads the older days."
  (when (buffer-live-p page)
    (with-current-buffer page
      (setq peta-journal--scroll-timer nil)
      (when-let* ((window (get-buffer-window (current-buffer)))
                  (top (peta-journal--row-index (peta-journal--id-at (window-start window))))
                  (end (min (window-end window t) (1- (point-max))))
                  (bottom (peta-journal--row-index (peta-journal--id-at end)))
                  (scrolled (cons top bottom)))
        (unless (equal scrolled peta-journal--scrolled)
          (setq peta-journal--scrolled scrolled)
          (peta-journal--send "scrolled" :top top :bottom bottom))))))

;;;; Starting and stopping

(defun peta-journal--program ()
  "The program to run: the one set, or on the `exec-path', or in the build folder."
  (or peta-journal-shell-program
      (executable-find "client-shell-emacs")
      (let* ((here (file-name-directory (locate-library "peta-journal")))
             (target (expand-file-name "../../../../target/" here)))
        (seq-find #'file-executable-p
                  (mapcan (lambda (profile)
                            (list (expand-file-name (concat profile "/client-shell-emacs.exe") target)
                                  (expand-file-name (concat profile "/client-shell-emacs") target)))
                          '("release" "debug"))))
      (user-error "Cannot find client-shell-emacs: build it or set `peta-journal-shell-program'")))

(defun peta-journal--start-shell (url)
  "Start the shell of the current buffer, and open URL in it."
  (let ((buffer (current-buffer)))
    (setq peta-journal--seq 0
          peta-journal--ack 0
          peta-journal--holding nil
          peta-journal--page-path nil
          peta-journal--properties nil
          peta-journal--free-keys nil
          peta-journal--datetime-keys nil
          peta-journal--output "")
    (setq peta-journal--process
          (make-process :name "client-shell-emacs"
                        :command (list (peta-journal--program) peta-journal-server)
                        :connection-type 'pipe
                        :coding 'utf-8-unix
                        :noquery t
                        :stderr (get-buffer-create "*peta-journal-log*")
                        :filter (lambda (_process output)
                                  (when (buffer-live-p buffer)
                                    (with-current-buffer buffer
                                      (peta-journal--read-output output))))
                        :sentinel #'peta-journal--sentinel))
    (process-put peta-journal--process 'peta-page buffer)
    (unless peta-journal--idle-timer
      (setq peta-journal--idle-timer
            (run-with-idle-timer peta-journal-idle-finish-seconds t #'peta-journal--idle)))
    (add-hook 'kill-emacs-hook #'peta-journal--stop-all)
    (peta-journal--send "start" :url url)))

(defun peta-journal--ensure-started ()
  "Start the shell and the buffer of the page, unless they are there.
Return the buffer of the page that a command is about."
  (let ((buffer (or (peta-journal--page)
                    (with-current-buffer (get-buffer-create "*PetaJournal*")
                      (peta-journal-mode)
                      (current-buffer)))))
    (with-current-buffer buffer
      (unless (process-live-p peta-journal--process)
        (peta-journal--start-shell "/today")))
    buffer))

;;;###autoload
(defun peta-journal ()
  "Open PetaJournal, on today's journal when it was not open yet."
  (interactive)
  (pop-to-buffer-same-window (peta-journal--ensure-started)))

(defun peta-journal--wait-for-page ()
  "Wait until the shell of the current buffer has sent the items of a page.
The core first sends a page with no rows.  A key pressed then would put its
character in front of the first bullet."
  (peta-journal--wait-until (lambda () (> (length peta-journal--rows) 0))))

(defun peta-journal--other-buffer-of-page (buffer)
  "Another buffer that shows the page of BUFFER, or nil."
  (when-let* ((id (buffer-local-value 'peta-journal--page-id buffer)))
    (seq-find (lambda (page)
                (and (not (eq page buffer))
                     (equal (buffer-local-value 'peta-journal--page-id page) id)))
              (peta-journal--pages))))

;;;###autoload
(defun peta-journal-new-buffer (path)
  "Open the page at PATH in a new buffer, with a shell of its own.
An empty PATH opens today's journal.  When a buffer shows the page already, go
to that buffer: edits of one item in two buffers could overwrite each other."
  (interactive
   (list (completing-read "Page in a new buffer: " (peta-journal--recent-pages) nil nil)))
  (let ((buffer (generate-new-buffer "*PetaJournal*")))
    (with-current-buffer buffer
      (peta-journal-mode)
      (peta-journal--start-shell (concat "/pages/" path))
      ;; No warning about a page shown twice: this command goes on to the other buffer
      (let ((inhibit-message t))
        (peta-journal--wait-for-page)))
    (let ((other (peta-journal--other-buffer-of-page buffer)))
      (when other
        (kill-buffer buffer))
      (pop-to-buffer-same-window (or other buffer)))))

(defun peta-journal--stop-all ()
  "Save what was typed in every buffer, and stop their shells."
  (dolist (page (peta-journal--pages))
    (with-current-buffer page
      (peta-journal-stop))))

(defun peta-journal-stop ()
  "Save what was typed in the current buffer and stop its shell."
  (interactive)
  (when (process-live-p peta-journal--process)
    (set-process-sentinel peta-journal--process #'ignore)
    ;; The shell ends the edit by itself when its input closes, and exits when it is saved
    (process-send-eof peta-journal--process)
    (let ((deadline (+ (float-time) peta-journal--stop-seconds)))
      (while (and (process-live-p peta-journal--process)
                  (< (float-time) deadline))
        (accept-process-output peta-journal--process 0.05))))
  (when (and peta-journal--idle-timer
             (not (seq-some (lambda (page)
                              (process-live-p (buffer-local-value 'peta-journal--process page)))
                            (peta-journal--pages))))
    (cancel-timer peta-journal--idle-timer)
    (setq peta-journal--idle-timer nil)))

(provide 'peta-journal)

;;; peta-journal.el ends here
