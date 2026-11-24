;;; peta-journal.el --- Emacs frontend for PetaJournal -*- lexical-binding: t; -*-

;; Package-Requires: ((emacs "29.1"))

;;; Commentary:

;; A thin view on the PetaJournal core.  The program `outliner-emacs' holds the core and talks
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
(require 'peta-journal-protocol)
(require 'subr-x)
(require 'text-property-search)

(defgroup peta-journal nil
  "Emacs frontend for PetaJournal."
  :group 'applications)

(defcustom peta-journal-server "http://localhost:8087"
  "The address of the journal server."
  :type 'string)

(defcustom peta-journal-shell-program nil
  "The program `outliner-emacs'.
When nil, it is looked for on the `exec-path' and in the build folder
of the repository."
  :type '(choice (const nil) file))

(defcustom peta-journal-idle-finish-seconds 2
  "How long Emacs may idle before the edit of the current item ends.
The core does not take in changes from elsewhere while an edit is open."
  :type 'number)

(defcustom peta-journal-image-memory-limit (* 64 1024 1024)
  "How many bytes of images Emacs keeps at most.
Over it, Emacs forgets the images that no open page links to, the one used
longest ago first.  A page that shows a forgotten image fetches it again."
  :type 'natnum)

(defcustom peta-journal-item-gap 4
  "The gap between two items, in pixels.
It applies to the rows drawn after a change.  Only a graphical Emacs shows it."
  :type 'natnum)

(defconst peta-journal--bullets
  '(("plain" . "•") ("transclusion" . "⊡") ("broken" . "✗") ("hidden" . " "))
  "The mark of each kind of row.
A transclusion has a square around its dot, as the web frontend draws it.
A hidden bullet is a space: the body stays where it is when the bullet comes
back.")

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

(defface peta-journal-selected '((t :inherit region))
  "The rows of the selected items.")

(defface peta-journal-sidebar-section '((t :inherit bold))
  "The titles of the sections of the sidebar.")

(defface peta-journal-sidebar-current '((t :inherit highlight))
  "The page on screen, in the list of its folder in the sidebar.")

(defface peta-journal-sidebar-folder '((t :inherit shadow))
  "The folder before the name of a recent page in the sidebar.")

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

(defface peta-journal-quote '((t :inherit font-lock-doc-face))
  "A block quote.")

(defface peta-journal-list-mark '((t :inherit font-lock-builtin-face))
  "The mark of a list item or of a task in a list.")

(defface peta-journal-rule '((t :inherit shadow))
  "A horizontal rule.")

(defface peta-journal-table '((t :inherit font-lock-type-face))
  "A table.")

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
(defvar-local peta-journal--sidebar nil "What the sidebar lists for the shown page.")
(defvar-local peta-journal--scroll-timer nil "The timer that reports the scroll position.")
(defvar-local peta-journal--rows [] "The rows on screen, as the shell sent them.")
(defvar-local peta-journal--page-id nil "The id of the page on screen.")
(defvar-local peta-journal--current nil "The id of the item being edited.")
(defvar-local peta-journal--markdown-item nil
  "The id of the item that shows its Markdown, where the point is (ADR 077).
The other items show their shown text.")
(defvar-local peta-journal--last-point nil "Where the point was after the last command.")
(defvar-local peta-journal--title "" "The title of the page on screen.")
(defvar-local peta-journal--page-path nil "The path of the page on screen, if it can be renamed.")

(defvar-local peta-journal--index-folder nil
  "The folder of the index page on screen, as `a/b', if it can be renamed.")

(defvar-local peta-journal--demote-name nil
  "The name the index page on screen takes when it is demoted, if it can be.")
(defvar-local peta-journal--properties nil
  "The property rows of the page on screen, or nil on a screen without properties.")
(defvar-local peta-journal--free-keys nil
  "The property keys that can still be added to the page.
Each is a plist of :key and :label.")
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

(defconst peta-journal--look-faces
  '(("bold" . peta-journal-bold) ("italic" . peta-journal-italic)
    ("struck" . peta-journal-strike) ("underlined" . peta-journal-underline)
    ("marked" . peta-journal-highlight) ("code" . peta-journal-code)
    ("raised" . peta-journal-script) ("lowered" . peta-journal-script)
    ("hidden" . peta-journal-spoiler)
    ("heading1" . peta-journal-heading) ("heading2" . peta-journal-heading)
    ("heading3" . peta-journal-heading) ("heading4" . peta-journal-heading)
    ("heading5" . peta-journal-heading) ("heading6" . peta-journal-heading)
    ("mark" . peta-journal-list-mark) ("quote" . peta-journal-quote)
    ("rule" . peta-journal-rule) ("code_block" . peta-journal-code)
    ("table_header" . peta-journal-table))
  "The face of each look of a span, by the names of
`peta-journal-protocol-emacs-look-names'.")

(defun peta-journal--span-faces (span)
  "The faces of the looks of SPAN."
  (mapcar (lambda (look) (cdr (assoc look peta-journal--look-faces)))
          (peta-journal-protocol-emacs-span-looks span)))

(defun peta-journal-follow-link (event)
  "Open the link of the shown text that EVENT clicked on."
  (interactive "e")
  (let ((link (get-text-property (posn-point (event-start event)) 'peta-link)))
    (peta-journal-protocol-handle-rich-link
     link
     (list :page (lambda (url) (peta-journal--go "go" :url url))
           :web #'browse-url
           :file (lambda (path)
                   (browse-url (concat (string-trim-right peta-journal-server "/") "/" path)))))))

(defvar peta-journal--link-map
  (let ((map (make-sparse-keymap)))
    ;; Not the point: it would come into the row, which then shows its Markdown and no link
    (define-key map [down-mouse-1] #'ignore)
    (define-key map [mouse-1] #'peta-journal-follow-link)
    map)
  "The keys on a link of the shown text.")

(defun peta-journal--shown-string (text)
  "The EmacsText TEXT as it goes into the buffer.
Without its last line break: the newline of the row ends it."
  (let* ((whole (peta-journal-protocol-emacs-text-text text))
         (end (if (string-suffix-p "\n" whole) (1- (length whole)) (length whole)))
         (shown (substring whole 0 end)))
    (seq-doseq (span (peta-journal-protocol-emacs-text-spans text))
      (add-text-properties (min end (peta-journal-protocol-emacs-span-from span))
                           (min end (peta-journal-protocol-emacs-span-to span))
                           (peta-journal--span-properties span)
                           shown))
    shown))

(defun peta-journal--span-properties (span)
  "The text properties of SPAN in the shown text."
  (let ((faces (peta-journal--span-faces span))
        (link (peta-journal-protocol-emacs-span-link span)))
    (append
     (when link
       (list 'peta-link link
             'keymap peta-journal--link-map
             'mouse-face 'highlight
             'help-echo "mouse-1: open the link"))
     (when (or faces link)
       (list 'font-lock-face (append faces (and link '(peta-journal-link)))))
     (when-let* ((image (peta-journal-protocol-emacs-span-image span)))
       (list 'peta-image image))
     (unless (peta-journal-protocol-emacs-span-letters span)
       (list 'peta-added t)))))

(defun peta-journal--body-string (row)
  "The body of ROW as it goes into the buffer.
The Markdown in the item of `peta-journal--markdown-item', or else the
shown text."
  (if (equal (plist-get row :id) peta-journal--markdown-item)
      (plist-get row :body)
    ;; Only the Markdown is sent to the core: a change here would be one to a text it does not know
    (propertize (peta-journal--shown-string (plist-get row :text)) 'read-only t)))

(defun peta-journal--redraw-item (id)
  "Write the row of the item ID again, as `peta-journal--body-string' gives it."
  (when-let* ((index (peta-journal--row-index id)))
    (let ((inhibit-read-only t)
          (inhibit-modification-hooks t))
      (save-excursion
        (peta-journal--write-row index (aref peta-journal--rows index))))
    (restore-buffer-modified-p nil)
    (peta-journal--draw-images-of id)))

(defun peta-journal--show-markdown (id)
  "Let the item ID show its Markdown, and the item that showed it its shown text."
  (let ((old peta-journal--markdown-item))
    (unless (equal id old)
      (setq peta-journal--markdown-item id)
      (when old
        (peta-journal--redraw-item old))
      (when id
        (peta-journal--redraw-item id)))))

(defun peta-journal--show-markdown-at-point ()
  "Let the item at the point show its Markdown, with the point at its start.
For a point that came into a row without a command, as when its item has gone.
The edit starts with the next command."
  (let ((id (peta-journal--id-at (point))))
    (unless (equal id peta-journal--markdown-item)
      (peta-journal--show-markdown id)
      (when id
        (peta-journal--goto-item id 0)))))

(defun peta-journal--letter-at (pos)
  "The number of shown letters in front of POS in its body.
What Rust added to the shown text, such as the mark of a list, is not a shown
letter (ADR 044)."
  (let ((start (peta-journal--body-start (peta-journal--row-start pos))))
    (cl-count-if-not (lambda (at) (get-text-property at 'peta-added))
                     (number-sequence start (1- pos)))))

(defun peta-journal--row-string (row)
  "The text of ROW, as it goes into the buffer."
  (let* ((prefix (peta-journal--body-prefix (plist-get row :level)))
         (body (propertize (peta-journal--body-string row) 'line-prefix prefix 'wrap-prefix prefix))
         (text (concat (peta-journal--header-string row)
                       (peta-journal--bullet-string row)
                       body
                       (peta-journal--newline-string))))
    (put-text-property 0 1 'peta-row (plist-get row :id) text)
    text))

(defun peta-journal--typed (ack)
  "The edited item and its text on screen, as (ID . BODY).
Nil unless ACK is behind the typing."
  (when (and peta-journal--current (< ack peta-journal--seq))
    (when-let* ((old (peta-journal--row-of peta-journal--current)))
      (cons peta-journal--current (plist-get old :body)))))

(defun peta-journal--keep-typed (row typed)
  "ROW, with the body TYPED from `peta-journal--typed' when ROW is its item."
  (if (and typed (equal (plist-get row :id) (car typed)))
      (plist-put (copy-sequence row) :body (cdr typed))
    row))

(defun peta-journal--row-end (index)
  "The end of the row at INDEX, which is the start of the next one."
  (peta-journal--nth-row (1+ index)))

(defun peta-journal--write-row (index row)
  "Write ROW in place of the row at INDEX, or at INDEX when no row is there."
  (let ((start (peta-journal--nth-row index)))
    (when (< index (length peta-journal--rows))
      (delete-region start (peta-journal--row-end index)))
    (goto-char start)
    (insert (peta-journal--row-string row))))

(defun peta-journal--change-handlers (typed)
  "What to do with each kind of change of the rows, with the fields of the change.
Each applies its change to the buffer and to `peta-journal--rows', and returns
the ids of the rows it wrote, or t when it wrote all.  TYPED is the body to
keep, from `peta-journal--typed'.  The places count in the rows as the changes
before left them.  The kinds and their fields are those of
`peta-journal-protocol-list-change-fields'."
  (list
   :insert
   (lambda (at item)
     (let ((row (peta-journal--keep-typed item typed))
           (rows peta-journal--rows))
       (goto-char (peta-journal--nth-row at))
       (insert (peta-journal--row-string row))
       (setq peta-journal--rows (vconcat (seq-subseq rows 0 at) (vector row)
                                         (seq-subseq rows at)))
       (list (peta-journal-protocol-row-view-id row))))
   :remove
   (lambda (at)
     (let ((rows peta-journal--rows))
       (delete-region (peta-journal--nth-row at) (peta-journal--row-end at))
       (setq peta-journal--rows (vconcat (seq-subseq rows 0 at) (seq-subseq rows (1+ at))))
       nil))
   :move
   (lambda (from to)
     (let* ((rows peta-journal--rows)
            (row (aref rows from))
            (start (peta-journal--nth-row from))
            (text (buffer-substring start (peta-journal--row-end from)))
            (others (vconcat (seq-subseq rows 0 from) (seq-subseq rows (1+ from)))))
       (delete-region start (peta-journal--row-end from))
       (goto-char (peta-journal--nth-row to))
       (insert text)
       (setq peta-journal--rows (vconcat (seq-subseq others 0 to) (vector row)
                                         (seq-subseq others to)))
       ;; The images of the moved row were overlays on the text that was deleted
       (list (peta-journal-protocol-row-view-id row))))
   :update
   (lambda (at item)
     (let ((row (peta-journal--keep-typed item typed))
           (old (aref peta-journal--rows at)))
       (setq peta-journal--rows (vconcat peta-journal--rows))
       (aset peta-journal--rows at row)
       ;; The text on screen is the same.  Writing it again would only move the point
       (unless (equal-including-properties (peta-journal--row-string row)
                                           (peta-journal--row-string old))
         (peta-journal--write-row at row)
         (list (peta-journal-protocol-row-view-id row)))))
   :reset
   (lambda (items)
     (let ((new (vconcat (mapcar (lambda (row) (peta-journal--keep-typed row typed)) items))))
       (erase-buffer)
       (seq-doseq (row new)
         (insert (peta-journal--row-string row)))
       (setq peta-journal--rows new)
       t))))

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

(defun peta-journal--apply-rows (changes page-id ack)
  "Apply the CHANGES of the rows of PAGE-ID in order (ADR 078).
The shell has handled the lines up to ACK.  Only the rows that a change names
are rewritten, so what does not change stays where it is."
  (let* ((typed (peta-journal--typed ack))
         (handlers (peta-journal--change-handlers typed))
         (new-page (not (equal page-id peta-journal--page-id)))
         (windows (get-buffer-window-list (current-buffer) nil t))
         (window-places (mapcar (lambda (w) (cons w (peta-journal--place-of (window-point w))))
                                windows))
         (place (peta-journal--place-of (point)))
         (written nil)
         (inhibit-read-only t)
         (inhibit-modification-hooks t))
    (save-excursion
      (seq-doseq (change changes)
        (let ((ids (peta-journal-protocol-handle-list-change change handlers)))
          (setq written (if (or (eq ids t) (eq written t)) t (append ids written))))))
    (setq peta-journal--page-id page-id)
    ;; The core has ended the edit of an item that is gone, for one deleted here or by
    ;; another client.  Telling it so again would be about an item it does not know
    (unless (and peta-journal--current (peta-journal--row-index peta-journal--current))
      (setq peta-journal--current nil))
    (when new-page
      (peta-journal--warn-if-shown-elsewhere)
      (peta-journal--forget-unknown-images))
    (restore-buffer-modified-p nil)
    (cond
     ((zerop (length peta-journal--rows)) (goto-char (point-min)))
     ((or new-page (null place))
      (goto-char (peta-journal--row-body-start 0))
      (setq peta-journal--current nil
            peta-journal--scrolled nil))
     (t
      (goto-char (peta-journal--pos-of-place place))
      (pcase-dolist (`(,window . ,window-place) window-places)
        (when window-place
          (set-window-point window (peta-journal--pos-of-place window-place))))))
    (peta-journal--show-markdown-at-point)
    (setq peta-journal--last-point (point))
    ;; After the point has its place: the item it is in keeps its text
    (if (eq written t)
        (peta-journal--draw-images (point-min) (point-max))
      (mapc #'peta-journal--draw-images-of (delete-dups written)))
    (peta-journal--show-selection)))

(defvar peta-journal-selection-mode)

(defun peta-journal--show-selection ()
  "Color the selected rows.
The buffer is in `peta-journal-selection-mode' while there are any."
  (remove-overlays (point-min) (point-max) 'peta-selection t)
  (let ((pos (point-min))
        (any nil))
    (seq-doseq (row peta-journal--rows)
      (let ((next (or (peta-journal--next-row pos) (point-max))))
        (when (plist-get row :selected)
          (setq any t)
          (let ((overlay (make-overlay pos next)))
            (overlay-put overlay 'peta-selection t)
            (overlay-put overlay 'face 'peta-journal-selected)))
        (setq pos next)))
    (unless (eq any peta-journal-selection-mode)
      (peta-journal-selection-mode (if any 1 -1)))))

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

;; Global: buffers of the same server show the same files
(defvar peta-journal--images (make-hash-table :test #'equal)
  "What is known of the images by URL: their bytes, `loading' or `missing'.")

(defun peta-journal--image-url (image)
  "The URL of the RichImage IMAGE on the server."
  (concat (string-trim-right peta-journal-server "/") "/"
          (peta-journal-protocol-rich-image-path image)))

(defvar peta-journal--image-uses (make-hash-table :test #'equal)
  "When each image was used last, by URL, as a count that only grows.
Not the time: two uses may come in the same instant.")

(defvar peta-journal--image-use-count 0
  "The count of the last use of an image.")

(defun peta-journal--use-image (url)
  "Note that the image at URL was used now."
  (puthash url (setq peta-journal--image-use-count (1+ peta-journal--image-use-count))
           peta-journal--image-uses))

(defun peta-journal--linked-image-urls ()
  "The URLs of the images that the open pages link to."
  (let ((urls (make-hash-table :test #'equal)))
    (dolist (page (peta-journal--pages))
      ;; From the rows: the item that shows its Markdown has its images too
      (seq-doseq (row (buffer-local-value 'peta-journal--rows page))
        (seq-doseq (span (peta-journal-protocol-emacs-text-spans (plist-get row :text)))
          (when-let* ((image (peta-journal-protocol-emacs-span-image span)))
            (puthash (peta-journal--image-url image) t urls)))))
    urls))

(defun peta-journal--trim-images ()
  "Forget images until their bytes are within `peta-journal-image-memory-limit'.
An image that an open page links to stays: a page would fetch it and draw it
again at once, and that could go on forever."
  (let ((linked (peta-journal--linked-image-urls))
        (total 0)
        (spare nil))
    (maphash (lambda (url known)
               (when (stringp known)
                 (setq total (+ total (string-bytes known)))
                 (unless (gethash url linked)
                   (push url spare))))
             peta-journal--images)
    (setq spare (sort spare (lambda (a b)
                              (< (gethash a peta-journal--image-uses 0)
                                 (gethash b peta-journal--image-uses 0)))))
    (while (and spare (> total peta-journal-image-memory-limit))
      (let ((url (pop spare)))
        (setq total (- total (string-bytes (gethash url peta-journal--images))))
        (remhash url peta-journal--images)
        (remhash url peta-journal--image-uses)))))

(defun peta-journal--image-bytes (url)
  "The bytes of the image at URL, or nil.  The first call starts to fetch them."
  (let ((known (gethash url peta-journal--images)))
    (cond
     ((stringp known)
      (peta-journal--use-image url)
      known)
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
  (when bytes
    (peta-journal--use-image url)
    (peta-journal--trim-images))
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

(defun peta-journal--shown-image (id image)
  "The Emacs image of the RichImage IMAGE in the item ID.
Nil while there is none."
  (when-let* ((bytes (peta-journal--image-bytes (peta-journal--image-url image))))
    (let ((width (peta-journal-protocol-rich-image-width image))
          (height (peta-journal-protocol-rich-image-height image)))
      ;; Without a type, this tells it from the bytes, and gives nil if it cannot tell it. A
      ;; type that this build of Emacs cannot show is an error, which would stop the process
      ;; filter: the text stays then
      (condition-case nil
          (apply #'create-image bytes nil t
                 (append (and width (list :width width))
                         (and height (list :height height))
                         (peta-journal--image-room id)))
        (error nil)))))

(defun peta-journal--on-window-size-change (_window)
  "Fit the images of the current buffer to the new size of its window."
  (peta-journal--draw-images (point-min) (point-max)))

(defun peta-journal--draw-images (start end)
  "Show the images of the shown text between START and END.
START and END are the edges of rows.  An image takes the place of the text
that stands for it, but for its line break.  An overlay leaves the text alone."
  (remove-overlays start end 'peta-image t)
  (when (display-images-p)
    (let ((pos start))
      (while (< pos end)
        (let ((next (next-single-property-change pos 'peta-image nil end)))
          (when-let* ((image (get-text-property pos 'peta-image))
                      (shown (peta-journal--shown-image (peta-journal--id-at pos) image)))
            (let ((overlay (make-overlay pos (if (eq (char-before next) ?\n) (1- next) next))))
              (overlay-put overlay 'peta-image t)
              (overlay-put overlay 'display shown)
              ;; The shell may rewrite the row, and nothing should stay behind
              (overlay-put overlay 'evaporate t)))
          (setq pos next))))))

(defun peta-journal--draw-images-of (id)
  "Show the images of the item ID, when it is there."
  (when-let* ((start (peta-journal--find-row id)))
    (peta-journal--draw-images start (peta-journal--body-end start))))

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

(defun peta-journal--on-frame-focus ()
  "Tell the shell of each page that the day may have changed.
Emacs calls this when the focus of a frame changes, as when it comes back
after the PC slept.  The core then opens the new day at once (#1031).
Nothing is told when the focus left Emacs."
  (when (seq-some #'frame-focus-state (frame-list))
    (dolist (page (peta-journal--pages))
      (with-current-buffer page
        (peta-journal--send "day_may_have_changed")))))

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

(defconst peta-journal--line-handlers
  (list
   :state (lambda (ack holding)
            (setq peta-journal--ack ack
                  peta-journal--holding holding))
   :rows (lambda (ack page-id changes) (peta-journal--apply-rows changes page-id ack))
   :focus (lambda (ack id cursor) (peta-journal--on-focus id cursor ack))
   :scroll #'peta-journal--on-scroll
   :title (lambda (title)
            (setq peta-journal--title title)
            (force-mode-line-update))
   :page-path (lambda (path) (setq peta-journal--page-path path))
   :index (lambda (folder demote-name)
            (setq peta-journal--index-folder folder
                  peta-journal--demote-name demote-name))
   :properties #'peta-journal--on-properties
   :status (lambda (connection)
             (setq peta-journal--connection connection)
             (force-mode-line-update))
   :search #'peta-journal--on-search
   :recent (lambda (paths) (setq peta-journal--recent (append paths nil)))
   :sidebar (lambda (index headings recent)
              (peta-journal--on-sidebar (list :index index :headings headings :recent recent)))
   :backlinks (lambda (groups) (setq peta-journal--backlinks groups))
   :message (lambda (text) (message "%s" text))
   :copy (lambda (text)
           (kill-new text)
           (message "Copied the selected items"))
   :page-names (lambda (query titles)
                 (setq peta-journal--page-names (cons query (append titles nil))))
   :ask #'peta-journal--on-ask)
  "What to do with each kind of line from the shell, with the fields of the line.
The kinds and their fields are those of `peta-journal-protocol-outbound-fields'.")

(defun peta-journal--on-line (message)
  "Do what the line MESSAGE from the shell of the current buffer says."
  (peta-journal-protocol-handle-outbound message peta-journal--line-handlers))

(defun peta-journal--on-focus (id cursor ack)
  "Put the point in the item ID at CURSOR.
The shell has handled the lines up to ACK.  Nothing happens when lines are
still under way: the point has moved on since."
  (when (>= ack peta-journal--seq)
    (let ((shown (and (peta-journal--find-row id)
                      (not (equal id peta-journal--markdown-item)))))
      (when shown
        (peta-journal--show-markdown id))
      (unless (and (null cursor) (not shown) (equal (peta-journal--id-at (point)) id))
        (peta-journal--goto-item id cursor)))
    (when (equal (peta-journal--id-at (point)) id)
      (setq peta-journal--current id
            peta-journal--last-point (point)))))

(defun peta-journal--on-scroll (id)
  "Show the item ID."
  (when (peta-journal--find-row id)
    (peta-journal--goto-item id 0)
    (setq peta-journal--last-point (point))
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
  "Tell the core that the item ID is being edited, in place of the one before.
The core ends the edit of the one before (ADR 057)."
  (unless (equal id peta-journal--current)
    (setq peta-journal--current id)
    (peta-journal--send "edit_requested" :id id :cursor (peta-journal--cursor))))

(defun peta-journal--finish-edit ()
  "End the edit of the current item, which saves it."
  (when peta-journal--current
    (peta-journal--send "edit_finished" :id peta-journal--current)
    (setq peta-journal--current nil)))

(defconst peta-journal--commands-that-end-the-edit
  '(peta-journal-save peta-journal-today peta-journal-back peta-journal-forward
    peta-journal-next-day peta-journal-previous-day peta-journal-open-at-point
    peta-journal-search peta-journal-rename-page peta-journal-demote-index
    peta-journal-delete-page)
  "The commands after which the point stays in an item that is not being edited.
A saved edit must stay closed, and a page being left has nothing more to edit.")

(defun peta-journal--post-command ()
  "Keep the point in a body, and the core informed of the item it is in."
  (peta-journal--keep-point-in-body)
  ;; The selected items are not edited: the core ended the edit when the selection began
  (unless (or peta-journal-selection-mode
              (memq this-command peta-journal--commands-that-end-the-edit))
    (when-let* ((id (peta-journal--id-at (point))))
      (if (equal id peta-journal--markdown-item)
          (peta-journal--start-edit id)
        (peta-journal--enter-row id))))
  (setq peta-journal--last-point (point)))

(defun peta-journal--enter-row (id)
  "Show the Markdown of the item ID that the point has come into, and edit it.
The point is on the shown text.  The shell turns the letters in front of it
into the place in the Markdown, and tells it with a focus (ADR 077)."
  (let ((letter (peta-journal--letter-at (point))))
    (peta-journal--show-markdown id)
    (peta-journal--goto-item id 0)
    (setq peta-journal--current id)
    (peta-journal--send-and-wait "edit_at_letter" :id id :letter letter)))

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
  "Let the core close the `[[' or the block opening just typed, or go over a `]'.
Only for the characters that can end an opening: waiting on every key would
slow typing down.  An HTML opening such as `<pre' ends in any character, so
every key counts on a line that starts with `<'."
  (when (or (memq (char-before) '(?\[ ?\] ?` ?~ ?>))
            (save-excursion
              (goto-char (max (line-beginning-position)
                              (peta-journal--body-start (peta-journal--row-start (point)))))
              (skip-chars-forward " ")
              (eq (char-after) ?<)))
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

(defun peta-journal-select ()
  "Select the item at the point, with its children.
While items are selected, the keys of `peta-journal-selection-mode' act on them
all: grow the selection, move, indent, delete or copy the items.  The text of
the buffer cannot change then."
  (interactive)
  (let ((id (peta-journal--item)))
    (peta-journal--finish-edit)
    (peta-journal--send-and-wait "select" :id id)))

(defmacro peta-journal--define-selection-key (command key doc)
  "Define COMMAND, which sends the selection key KEY to the core.
DOC says what it does."
  `(defun ,command ()
     ,doc
     (interactive)
     (peta-journal--send-and-wait "selection_key" :key ,key)))

(peta-journal--define-selection-key peta-journal-selection-down "ExtendDown"
  "Add the item below to the selection, or take the top one away.")
(peta-journal--define-selection-key peta-journal-selection-up "ExtendUp"
  "Add the item above to the selection, or take the bottom one away.")
(peta-journal--define-selection-key peta-journal-selection-move-up "MoveUp"
  "Move the selected items up.")
(peta-journal--define-selection-key peta-journal-selection-move-down "MoveDown"
  "Move the selected items down.")
(peta-journal--define-selection-key peta-journal-selection-indent "Indent"
  "Indent the selected items.")
(peta-journal--define-selection-key peta-journal-selection-outdent "Outdent"
  "Outdent the selected items.")
(peta-journal--define-selection-key peta-journal-selection-delete "Delete"
  "Delete the selected items, with their children.")
(peta-journal--define-selection-key peta-journal-selection-copy "Copy"
  "Put the Markdown of the selected items in the kill ring.")
(peta-journal--define-selection-key peta-journal-selection-cut "Cut"
  "Put the Markdown of the selected items in the kill ring, and delete them.")
(peta-journal--define-selection-key peta-journal-selection-undo "Undo"
  "Undo the change made last.")
(peta-journal--define-selection-key peta-journal-selection-redo "Redo"
  "Redo the change undone last.")
(peta-journal--define-selection-key peta-journal-selection-end "Escape"
  "End the selection.  The items stay as they are.")

(defvar-keymap peta-journal-selection-mode-map
  :doc "The keys while items are selected."
  "n" #'peta-journal-selection-down
  "C-n" #'peta-journal-selection-down
  "<down>" #'peta-journal-selection-down
  "S-<down>" #'peta-journal-selection-down
  "p" #'peta-journal-selection-up
  "C-p" #'peta-journal-selection-up
  "<up>" #'peta-journal-selection-up
  "S-<up>" #'peta-journal-selection-up
  "M-S-<up>" #'peta-journal-selection-move-up
  "M-S-<down>" #'peta-journal-selection-move-down
  "TAB" #'peta-journal-selection-indent
  "M-S-<right>" #'peta-journal-selection-indent
  "<backtab>" #'peta-journal-selection-outdent
  "M-S-<left>" #'peta-journal-selection-outdent
  "d" #'peta-journal-selection-delete
  "DEL" #'peta-journal-selection-delete
  "C-d" #'peta-journal-selection-delete
  "<delete>" #'peta-journal-selection-delete
  "w" #'peta-journal-selection-copy
  "M-w" #'peta-journal-selection-copy
  "C-w" #'peta-journal-selection-cut
  "C-/" #'peta-journal-selection-undo
  "<remap> <undo>" #'peta-journal-selection-undo
  "C-?" #'peta-journal-selection-redo
  "<remap> <undo-redo>" #'peta-journal-selection-redo
  "C-g" #'peta-journal-selection-end
  "q" #'peta-journal-selection-end
  "<escape>" #'peta-journal-selection-end)

(define-minor-mode peta-journal-selection-mode
  "The mode while items are selected, with \[peta-journal-select].
The text of the buffer cannot change, and these keys act on the selected items:
\{peta-journal-selection-mode-map}
The mode follows the selection in the core: it is on while any row is selected."
  :lighter " Selected"
  (setq buffer-read-only peta-journal-selection-mode))

(defun peta-journal-copy-reference ()
  "Put the id of the item at the point in the kill ring.
Paste it with \[peta-journal-paste-as-transclusion] to show the item in another place."
  (interactive)
  (let ((id (peta-journal--item)))
    (kill-new id)
    (message "Copied the reference %s" id)))

(defun peta-journal-paste-as-transclusion ()
  "Make the item at the point show the item whose id is the latest kill.
The kill can come from \[peta-journal-copy-reference], or from \"Copy Reference\"
of the web app through the clipboard.  Text that is not an item id does nothing."
  (interactive)
  (let ((text (ignore-error error (current-kill 0 t))))
    (unless text
      (user-error "Nothing to paste: copy the reference of an item first"))
    (peta-journal--send-and-wait "paste_as_transclusion"
                                 :id (peta-journal--item) :text text)))

(defun peta-journal-remove-transclusion ()
  "Make the transclusion at the point a plain item again."
  (interactive)
  (let ((id (peta-journal--item)))
    (unless (member (plist-get (peta-journal--row-of id) :bullet) '("transclusion" "broken"))
      (user-error "The item is not a transclusion"))
    (peta-journal--send-and-wait "remove_transclusion" :id id)))

(defun peta-journal-send-to-stash ()
  "Send the item at the point, with its children, to the stash of the page.
The stash is the `# Stash' heading at the end of the page, which the server
makes when there is none.  The text typed into the item goes with it."
  (interactive)
  (peta-journal--send-and-wait "send_to_stash" :id (peta-journal--item)))

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

(defun peta-journal-rename-page (path &optional folder)
  "Give the page on screen the new PATH.  A `/' in PATH puts the page in a folder.
On an index page, PATH is the new path of its folder, and FOLDER is non-nil."
  (interactive
   (let* ((page (peta-journal--page))
          (current (and page (buffer-local-value 'peta-journal--page-path page)))
          (index (and page (buffer-local-value 'peta-journal--index-folder page))))
     (cond
      (current (list (read-string "Rename page to: " current) nil))
      (index (list (read-string "Rename folder to: " index) t))
      (t (user-error "A journal, the root index or an atlas cannot be renamed")))))
  (if folder
      (peta-journal--go "rename_folder" :path path)
    (peta-journal--go "rename_page" :path path)))

(defun peta-journal-demote-index ()
  "Make the index page on screen the page of the name of its folder.
Only an index page without pages under it can be demoted.  Emacs asks first."
  (interactive)
  (let* ((page (peta-journal--page))
         (folder (and page (buffer-local-value 'peta-journal--index-folder page)))
         (name (and page (buffer-local-value 'peta-journal--demote-name page))))
    (unless (and folder name)
      (user-error "Only an index page without pages under it can become a page"))
    (when (yes-or-no-p (format "Make the index page of %s/ the page %s? "
                               folder (concat (file-name-directory folder) name)))
      (peta-journal--go "demote_index"))))

(defun peta-journal-delete-page ()
  "Delete the page on screen, after a question.  Today's journal opens then.
Today's journal itself cannot be deleted."
  (interactive)
  (peta-journal--go "delete_page"))

(defun peta-journal-back ()
  "Go back to the page shown before."
  (interactive)
  (peta-journal--go "back"))

(defun peta-journal-forward ()
  "Go forward again to the page left by going back."
  (interactive)
  (peta-journal--go "forward"))

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
  (let ((label (plist-get entry :label))
        (snippet (peta-journal--strip-tags (plist-get entry :snippet))))
    (if (equal (plist-get entry :kind) "item")
        (format "%s: %s" label (string-replace "\n" " " snippet))
      label)))

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
       (vconcat (mapcar (lambda (path)
                          (list :kind "recent" :title path :label path :snippet "" :path path))
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

;;;; Sidebar

(defcustom peta-journal-sidebar-width 30
  "The width of the sidebar, in columns."
  :type 'natnum)

(defcustom peta-journal-sidebar-collapsed nil
  "The sections of the sidebar that show only their title.
TAB in the sidebar opens or closes the section at the point, and saves this."
  :type '(set (const :tag "The pages of the folder" index)
              (const :tag "Contents" contents)
              (const :tag "Recent Pages" recent)))

(defconst peta-journal--sidebar-buffer "*PetaJournal Sidebar*"
  "The name of the buffer of the sidebar.")

(defvar-local peta-journal--sidebar-page nil
  "In the sidebar: the buffer of the page whose lists it shows.")

(defvar-keymap peta-journal-sidebar-mode-map
  :parent special-mode-map
  "RET" #'peta-journal-sidebar-open
  "TAB" #'peta-journal-sidebar-toggle-section
  "<mouse-2>" #'peta-journal-sidebar-mouse-open
  "n" #'next-line
  "p" #'previous-line)

(define-derived-mode peta-journal-sidebar-mode special-mode "PetaJournal Sidebar"
  "The sidebar of a page, as in the web frontend: the pages of its folder, its
headings and the recent pages.  RET opens the line at the point, and TAB
opens or closes its section."
  (setq truncate-lines t))

(defun peta-journal-sidebar ()
  "Show the sidebar on the left of the frame, or hide it when it is shown."
  (interactive)
  (if-let* ((window (get-buffer-window peta-journal--sidebar-buffer)))
      (delete-window window)
    (let ((page (peta-journal--page)))
      (unless page
        (user-error "No PetaJournal page"))
      (peta-journal--sidebar-render page)
      (display-buffer-in-side-window
       (get-buffer peta-journal--sidebar-buffer)
       `((side . left) (slot . 0) (window-width . ,peta-journal-sidebar-width)
         (preserve-size . (t . nil))
         (window-parameters . ((no-delete-other-windows . t))))))))

(defun peta-journal--on-sidebar (message)
  "Keep the lists of MESSAGE for the sidebar of the current page.
A shown sidebar of this page is drawn again; so is one of another page, when
this page is in the selected window, as the sidebar follows the page worked on."
  (setq peta-journal--sidebar message)
  (let ((page (current-buffer))
        (sidebar (get-buffer peta-journal--sidebar-buffer)))
    (when (and sidebar (get-buffer-window sidebar)
               (or (eq (buffer-local-value 'peta-journal--sidebar-page sidebar) page)
                   (eq (window-buffer (selected-window)) page)))
      (peta-journal--sidebar-render page))))

(defun peta-journal--sidebar-render (page)
  "Draw the sidebar with the lists of PAGE.  The point stays on its line."
  (let ((sidebar (buffer-local-value 'peta-journal--sidebar page)))
    (with-current-buffer (get-buffer-create peta-journal--sidebar-buffer)
      (unless (derived-mode-p 'peta-journal-sidebar-mode)
        (peta-journal-sidebar-mode))
      (setq peta-journal--sidebar-page page)
      (let ((inhibit-read-only t)
            (shown-line (line-number-at-pos)))
        (erase-buffer)
        (when-let* ((index (plist-get sidebar :index)))
          (peta-journal--sidebar-section
           'index (plist-get index :header) (peta-journal--sidebar-opener page (plist-get index :path))
           (lambda ()
             (when-let* ((parent (plist-get index :parent)))
               (peta-journal--sidebar-page-line page "  ../" parent))
             (seq-doseq (line (plist-get index :pages))
               (peta-journal--sidebar-page-line
                page (concat "  " (plist-get line :label)) (plist-get line :path)
                (and (plist-get line :current) 'peta-journal-sidebar-current))))))
        (let ((headings (plist-get sidebar :headings)))
          (unless (seq-empty-p headings)
            (peta-journal--sidebar-section
             'contents "Contents" #'peta-journal-sidebar-toggle-section
             (lambda ()
               (seq-doseq (heading headings)
                 (peta-journal--sidebar-heading-line page heading))))))
        (let ((recent (plist-get sidebar :recent)))
          (unless (seq-empty-p recent)
            (peta-journal--sidebar-section
             'recent "Recent Pages" #'peta-journal-sidebar-toggle-section
             (lambda ()
               (seq-doseq (line recent)
                 (peta-journal--sidebar-page-line
                  page (concat "  " (if-let* ((dir (plist-get line :dir)))
                                        (propertize dir 'face 'peta-journal-sidebar-folder)
                                      "")
                               (plist-get line :label))
                  (plist-get line :path)))))))
        (goto-char (point-min))
        (forward-line (1- shown-line))))))

(defvar-keymap peta-journal--sidebar-mark-map
  :doc "The keys on the mark before the title of a section of the sidebar."
  "<mouse-2>" #'peta-journal-sidebar-mouse-toggle)

(defun peta-journal--sidebar-section (section title action insert-lines)
  "Insert the SECTION of the sidebar: the line TITLE, which does ACTION when it is
opened, and under it what INSERT-LINES inserts, unless the section is closed.
A click on the mark before TITLE opens or closes the section."
  (let ((start (point))
        (open (not (memq section peta-journal-sidebar-collapsed))))
    (peta-journal--sidebar-line
     (concat (propertize (if open "▾ " "▸ ") 'keymap peta-journal--sidebar-mark-map
                         'help-echo "mouse-1: open or close the section")
             (propertize title 'face 'peta-journal-sidebar-section))
     action)
    (when open
      (funcall insert-lines))
    (put-text-property start (point) 'peta-section section)))

(defun peta-journal--sidebar-opener (page path)
  "What opens PATH in PAGE from a line of the sidebar."
  (lambda ()
    (peta-journal--sidebar-to-page page)
    (peta-journal--go "go_to_page" :path path)))

(defconst peta-journal--sidebar-sections '(index contents recent)
  "The sections of the sidebar, in their order.")

(defun peta-journal-sidebar-toggle-section ()
  "Close the section of the sidebar at the point, or open it when it is closed.
The closed sections are saved, so that they stay closed in the next Emacs."
  (interactive)
  (let ((section (get-text-property (point) 'peta-section)))
    (unless section
      (user-error "No section here"))
    (let* ((closed (if (memq section peta-journal-sidebar-collapsed)
                       (remq section peta-journal-sidebar-collapsed)
                     (cons section peta-journal-sidebar-collapsed)))
           (sorted (seq-filter (lambda (s) (memq s closed)) peta-journal--sidebar-sections)))
      ;; Emacs started with -q has no init file to save to, and refuses to overwrite one
      (if (or custom-file user-init-file)
          (customize-save-variable 'peta-journal-sidebar-collapsed sorted)
        (customize-set-variable 'peta-journal-sidebar-collapsed sorted)))
    (peta-journal--sidebar-render peta-journal--sidebar-page)
    (goto-char (text-property-any (point-min) (point-max) 'peta-section section))))

(defun peta-journal-sidebar-mouse-toggle (event)
  "Open or close the section of the sidebar whose mark EVENT clicked."
  (interactive "e")
  (mouse-set-point event)
  (peta-journal-sidebar-toggle-section))

(defun peta-journal--sidebar-page-line (page text path &optional face)
  "Insert the line TEXT of the sidebar, with FACE, which opens PATH in PAGE."
  (peta-journal--sidebar-line (if face (propertize text 'face face) text)
                              (peta-journal--sidebar-opener page path)))

(defun peta-journal--sidebar-heading-line (page heading)
  "Insert the line of HEADING of the sidebar, which moves the point of PAGE to it."
  (let ((id (plist-get heading :id)))
    (peta-journal--sidebar-line
     (concat (make-string (* 2 (plist-get heading :level)) ?\s) (plist-get heading :text))
     (lambda ()
       (peta-journal--sidebar-to-page page)
       (peta-journal--goto-item id 0)))))

(defun peta-journal--sidebar-line (text action)
  "Insert TEXT as a line of the sidebar, which does ACTION when it is opened."
  (insert (propertize text 'peta-action action 'mouse-face 'highlight 'follow-link t)
          "\n"))

(defun peta-journal--sidebar-to-page (page)
  "Make PAGE current, in its window when it has one, for a line of the sidebar."
  (when-let* ((window (get-buffer-window page)))
    (select-window window))
  (set-buffer page))

(defun peta-journal-sidebar-open ()
  "Open the line of the sidebar at the point."
  (interactive)
  (let ((action (get-text-property (point) 'peta-action)))
    (unless action
      (user-error "No entry here"))
    (funcall action)))

(defun peta-journal-sidebar-mouse-open (event)
  "Open the line of the sidebar that EVENT clicked."
  (interactive "e")
  (mouse-set-point event)
  (peta-journal-sidebar-open))

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
             (width (seq-max (cons 0 (mapcar (lambda (row) (string-width (plist-get row :label)))
                                             rows)))))
        (erase-buffer)
        (insert "Properties\n\n")
        (seq-doseq (row rows)
          (let* ((key (plist-get row :key))
                 (label (plist-get row :label))
                 (value (plist-get row :value)))
            (insert (propertize (format "%s%s  %s\n"
                                        label
                                        (make-string (- width (string-width label)) ?\s)
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

(defun peta-journal--choices (keys)
  "The KEYS, plists of :key and :label, as an alist from each label to its key."
  (mapcar (lambda (key) (cons (plist-get key :label) (plist-get key :key))) keys))

(defun peta-journal--property-choices (page field)
  "The labels and keys of the properties of the buffer PAGE whose FIELD is true.
See `peta-journal--choices'."
  (peta-journal--choices
   (seq-filter (lambda (row) (plist-get row field))
               (buffer-local-value 'peta-journal--properties page))))

(defun peta-journal--read-property-key (prompt choices)
  "Ask with PROMPT for the label of one of CHOICES, and return its key.
CHOICES is an alist from each label to its key.  The label shows the body of the
name and the type, so two keys that share a body are told apart.  Signals an
error when the answer is empty.  In the list of properties, the key on the line
of the point is the default."
  (let* ((here (get-text-property (line-beginning-position) 'peta-property-key))
         (label (completing-read prompt choices nil t nil nil (car (rassoc here choices)))))
    (when (string-empty-p label)
      (user-error "No property was chosen"))
    (cdr (assoc label choices))))

(defun peta-journal--read-property-to-set ()
  "Ask for a key and its value.  Return them as a list."
  (let* ((page (peta-journal--properties-page))
         (key (peta-journal--read-property-key
               "Set property: "
               (append (peta-journal--property-choices page :editable)
                       (peta-journal--choices
                        (buffer-local-value 'peta-journal--free-keys page)))))
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
   (let ((keys (peta-journal--property-choices (peta-journal--properties-page) :removable)))
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
  "C-c C-x f" #'peta-journal-forward
  "C-c C-x r" #'peta-journal-rename-page
  "C-c SPC" #'peta-journal-select
  "C-c C-x w" #'peta-journal-copy-reference
  "C-c C-x t" #'peta-journal-paste-as-transclusion
  "C-c C-x T" #'peta-journal-remove-transclusion
  "C-c C-x D" #'peta-journal-demote-index
  "C-c C-x s" #'peta-journal-sidebar
  "C-c C-x S" #'peta-journal-send-to-stash
  "C-c C-x d" #'peta-journal-delete-page
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
  ;; No keywords: the shell gives the colors (ADR 077).  Font Lock still shows `font-lock-face'
  (setq-local font-lock-defaults '(nil t))
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
      (executable-find "outliner-emacs")
      (let* ((here (file-name-directory (locate-library "peta-journal")))
             (target (expand-file-name "../../../../target/" here)))
        (seq-find #'file-executable-p
                  (mapcan (lambda (profile)
                            (list (expand-file-name (concat profile "/outliner-emacs.exe") target)
                                  (expand-file-name (concat profile "/outliner-emacs") target)))
                          '("release" "debug"))))
      (user-error "Cannot find outliner-emacs: build it or set `peta-journal-shell-program'")))

(defun peta-journal--start-shell (url &optional path)
  "Start the shell of the current buffer, and open URL in it.
With PATH, open the page of that name instead.  The shell makes its URL, so the
rule of encoding a name stays in one place."
  (let ((buffer (current-buffer)))
    (setq peta-journal--seq 0
          peta-journal--ack 0
          peta-journal--holding nil
          peta-journal--page-path nil
          peta-journal--index-folder nil
          peta-journal--demote-name nil
          peta-journal--properties nil
          peta-journal--free-keys nil
          peta-journal--datetime-keys nil
          peta-journal--output "")
    (setq peta-journal--process
          (make-process :name "outliner-emacs"
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
    (add-function :after after-focus-change-function #'peta-journal--on-frame-focus)
    (if path
        (peta-journal--send "start_at_page" :path path)
      (peta-journal--send "start" :url url))))

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
      (peta-journal--start-shell nil path)
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
