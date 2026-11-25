;;; peta-journal-e2e.el --- Emacs, the shell and a real server together -*- lexical-binding: t; -*-

;;; Commentary:

;; Starts a real journal server in a temporary folder and drives the real shell with real keys.
;; It runs what the other tests cannot: runtime.rs, the JSON lines and the save to the server.
;;
;; Build both programs first, then run from client/apps/emacs/outliner:
;;   emacs -Q --batch -L lisp -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit
;;
;; PETA_E2E_SERVER is the journal-server program.  The tests are skipped when it is missing.
;;
;; See ADR-002: the server is a throw-away one, with all it writes under one folder.  Otherwise it
;; would export into the real notes of the user.  The tests also stop when the server reports a
;; markdown folder outside that folder.

;;; Code:

(require 'ert)
(require 'cl-lib)
(require 'json)
(require 'url-util)
(require 'peta-journal)

(defconst peta-journal-e2e--port 18087 "The port of the test server.")

(defconst peta-journal-e2e--url (format "http://127.0.0.1:%d" peta-journal-e2e--port)
  "The address of the test server.")

(defconst peta-journal-e2e--here
  (file-name-directory (or load-file-name buffer-file-name))
  "The folder of this file.")

(defun peta-journal-e2e--server-program ()
  "The journal server to test against, or nil."
  (let ((program (getenv "PETA_E2E_SERVER")))
    (and program (file-executable-p program) program)))

(defun peta-journal-e2e--shell-program ()
  "The shell built from this repository, or nil."
  (let ((target (expand-file-name "../../../../../target/debug/" peta-journal-e2e--here)))
    (seq-find #'file-executable-p
              (list (expand-file-name "outliner-emacs.exe" target)
                    (expand-file-name "outliner-emacs" target)))))

(defun peta-journal-e2e--wait-for (what predicate)
  "Handle output until PREDICATE holds.  Fails with WHAT after 15 seconds."
  (let ((deadline (+ (float-time) 15)))
    (while (and (not (funcall predicate)) (< (float-time) deadline))
      (accept-process-output nil 0.05))
    (unless (funcall predicate)
      (error "Timed out waiting for %s.  Buffer: %S" what (buffer-string)))))

(defun peta-journal-e2e--request (method path &optional body)
  "Send METHOD to PATH of the test server with the JSON BODY.
Return the answer as a plist.  Not with url.el: its sentinel signals an error
when the server is not up yet, which ends a batch run.  A refused connection
signals here, where the caller can handle it."
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (let* ((payload (if body (encode-coding-string (json-serialize body) 'utf-8) ""))
           (process (open-network-stream "peta-e2e-http" (current-buffer)
                                         "127.0.0.1" peta-journal-e2e--port))
           (deadline (+ (float-time) 5)))
      (set-process-query-on-exit-flag process nil)
      (set-process-sentinel process #'ignore)
      (set-process-coding-system process 'binary 'binary)
      ;; HTTP/1.0: the server closes the connection at the end, and does not send chunks
      (process-send-string
       process
       ;; The server refuses a change of a client that does not name itself (ADR 015)
       (format (concat "%s %s HTTP/1.0\r\nContent-Type: application/json\r\n"
                       "X-Client-Id: e2e00000-0000-4000-8000-000000000000\r\n"
                       "Content-Length: %d\r\n\r\n%s")
               method (url-encode-url path) (length payload) payload))
      (while (and (process-live-p process) (< (float-time) deadline))
        (accept-process-output process 0.02))
      (delete-process process)
      (goto-char (point-min))
      (when (and (re-search-forward "\r?\n\r?\n" nil t) (not (eobp)))
        (json-parse-string (decode-coding-string (buffer-substring (point) (point-max)) 'utf-8)
                           :object-type 'plist :null-object nil :false-object nil)))))

(defun peta-journal-e2e--server-up-p ()
  "Non-nil when the test server answers."
  (ignore-errors (peta-journal-e2e--request "GET" "/api/config")))

(defun peta-journal-e2e--inside-p (path folder)
  "Non-nil when PATH is inside FOLDER."
  (string-prefix-p (file-name-as-directory (file-truename folder))
                   (file-name-as-directory (file-truename path))
                   t))

(defun peta-journal-e2e--write-config (root)
  "Write the config of the server in ROOT.
It names no folder of notes: a throw-away server has `markdown/' under its folder."
  (let ((config (expand-file-name "config" root)))
    (make-directory config t)
    (with-temp-file (expand-file-name "config.json" config)
      (insert (json-serialize (list :weekNumbering "iso"))))))

(defun peta-journal-e2e--start-server (root)
  "Start the test server with its files in ROOT, and return its process.
Signals an error when its notes would not stay inside ROOT."
  (peta-journal-e2e--write-config root)
  (let* ((process
          (make-process
           :name "peta-e2e-server"
           :buffer (get-buffer-create " *peta-e2e-server*")
           :noquery t
           :command (list (peta-journal-e2e--server-program)
                          "--port" (number-to-string peta-journal-e2e--port)
                          "--throwaway-dir" (expand-file-name root)))))
    (peta-journal-e2e--wait-for "the server" #'peta-journal-e2e--server-up-p)
    (let ((in-use (plist-get (peta-journal-e2e--request "GET" "/api/config") :markdownDir)))
      (unless (and in-use (peta-journal-e2e--inside-p in-use root))
        (delete-process process)
        (error "The test server writes to %s, outside %s" in-use root)))
    process))

(defun peta-journal-e2e--stop-server (server root)
  "Stop SERVER and remove ROOT.
On Windows, its files stay locked for a moment after it ends."
  (delete-process server)
  (let ((deadline (+ (float-time) 5))
        (removed nil))
    (while (and (not removed) (< (float-time) deadline))
      (condition-case nil
          (progn (delete-directory root t)
                 (setq removed t))
        (file-error (sleep-for 0.1))))))

(defmacro peta-journal-e2e--with-app (&rest body)
  "Run BODY in the page buffer of the real shell.
The server is a real one, with its files in a temporary folder."
  (declare (indent 0))
  `(progn
     (skip-unless (peta-journal-e2e--server-program))
     (skip-unless (peta-journal-e2e--shell-program))
     (should-not (peta-journal-e2e--server-up-p))
     (let* ((root (make-temp-file "peta-e2e" t))
            (peta-journal-server peta-journal-e2e--url)
            (peta-journal-shell-program (peta-journal-e2e--shell-program))
            (server (peta-journal-e2e--start-server root)))
       (unwind-protect
           (progn
             (peta-journal)
             (set-window-buffer (selected-window) "*PetaJournal*")
             (with-current-buffer "*PetaJournal*"
               (peta-journal-e2e--wait-for "the first page"
                                           (lambda () (> (length peta-journal--rows) 0)))
               ,@body))
         (mapc #'kill-buffer (peta-journal--pages))
         (peta-journal-stop)
         (peta-journal-e2e--stop-server server root)))))

(defun peta-journal-e2e--shown ()
  "The text of the buffer, with the space behind each bullet as a plain space."
  (string-replace " " " " (buffer-string)))

(defun peta-journal-e2e--keys (keys)
  "Press KEYS, through the same command loop as a person."
  (execute-kbd-macro (kbd keys))
  (peta-journal--wait))

(defun peta-journal-e2e--type (text)
  "Type TEXT, one key at a time."
  (execute-kbd-macro text)
  (peta-journal--wait))

(defun peta-journal-e2e--saved-items ()
  "The items of the page on screen, as the server has them."
  (plist-get (peta-journal-e2e--request "GET" (concat "/api/pages/" peta-journal--page-id))
             :items))

(defun peta-journal-e2e--saved-bodies ()
  "The bodies of the page on screen, as the server has them."
  (mapcar (lambda (item) (plist-get item :body)) (peta-journal-e2e--saved-items)))

(defun peta-journal-e2e--wait-saved (bodies)
  "Wait until the server has exactly BODIES on the page on screen."
  (peta-journal-e2e--wait-for
   (format "the server to have %S, has %S" bodies (peta-journal-e2e--saved-bodies))
   (lambda () (equal (peta-journal-e2e--saved-bodies) bodies))))

(defun peta-journal-e2e--file-with (folder text)
  "The markdown file in FOLDER that holds TEXT, or nil."
  (seq-find (lambda (file)
              (with-temp-buffer
                ;; The server may move a file away after the list is made, as a rename does
                (ignore-error file-missing
                  ;; The server writes UTF-8. Emacs on a Japanese Windows would guess Shift_JIS
                  (let ((coding-system-for-read 'utf-8))
                    (insert-file-contents file))
                  (search-forward text nil t))))
            (directory-files-recursively folder "\\.md\\'")))

(defun peta-journal-e2e--wait-for-file (folder text)
  "Wait until a markdown file in FOLDER holds TEXT, and return the file."
  (peta-journal-e2e--wait-for
   (format "a file with %s" text)
   (lambda () (peta-journal-e2e--file-with folder text)))
  (peta-journal-e2e--file-with folder text))

;; 今日のジャーナルが開き、打った文字が保存される。
(ert-deftest peta-journal-e2e-typed-text-is-saved ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "朝の散歩")
    (should (equal (peta-journal--body-at (point)) "朝の散歩"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("朝の散歩"))))

;; pointのない行は、シェルが送る表示の形で、色つきで見える。その行に入ると、Markdownの中の同じ文字の位置で編集が始まる（ADR 077）。
(ert-deftest peta-journal-e2e-a-row-shows-its-shown-text-until-the-point-enters-it ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "x")
    (peta-journal-e2e--keys "RET")
    (peta-journal-e2e--type "**太字**の後")
    (peta-journal-e2e--keys "C-p")
    (should (equal (peta-journal-e2e--shown) "• x\n• 太字の後\n"))
    (save-excursion
      (goto-char (point-min))
      (search-forward "太字")
      (should (memq 'peta-journal-bold (get-text-property (match-beginning 0) 'font-lock-face))))
    ;; Over the line break and the bullet, to the front of 後
    (peta-journal-e2e--keys "C-u 6 C-f")
    (should (equal (peta-journal-e2e--shown) "• x\n• **太字**の後\n"))
    (should (= (peta-journal--cursor) 7))))

;; Enterで項目が分かれ、カーソルは新しい項目の先頭へ移る。
(ert-deftest peta-journal-e2e-enter-splits-the-item ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "abcd")
    (peta-journal-e2e--keys "C-b C-b RET")
    (should (equal (peta-journal-e2e--shown) "• ab\n• cd\n"))
    (should (equal (peta-journal--body-at (point)) "cd"))
    (should (= (peta-journal--cursor) 0))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("ab" "cd"))))

;; 動いているシェルが止まると、警告が出る。
(ert-deftest peta-journal-e2e-a-stopped-shell-is-reported ()
  (peta-journal-e2e--with-app
    (let ((warning nil))
      (cl-letf (((symbol-function 'display-warning)
                 (lambda (_type message &rest _) (setq warning message))))
        (delete-process peta-journal--process)
        (peta-journal-e2e--wait-for "the warning" (lambda () warning)))
      (should (equal peta-journal--connection "shell stopped")))))

;; `[[`を打つと`]]`が補われ、続けて打った文字はその間に入る。
(ert-deftest peta-journal-e2e-a-wiki-link-closes-itself ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "see [[Plan")
    (should (equal (peta-journal--body-at (point)) "see [[Plan]]"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("see [[Plan]]"))))

;; `]]`まで手で打っても`]`は増えず、カーソルはリンクの後ろへ進む。
(ert-deftest peta-journal-e2e-the-closing-of-a-wiki-link-can-be-typed-over ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "see [[Plan]] now")
    (should (equal (peta-journal--body-at (point)) "see [[Plan]] now"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("see [[Plan]] now"))))

;; ```を打つと閉じるフェンスが補われる。中のEnterは項目を分けず、文字は次の行に入る。
(ert-deftest peta-journal-e2e-a-code-fence-closes-itself ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "```")
    (peta-journal-e2e--keys "RET")
    (peta-journal-e2e--type "code")
    (should (equal (peta-journal--body-at (point)) "```\ncode\n```"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("```\ncode\n```"))))

;; コードフェンスでなくても、開始記号を打つと閉じる記号が補われる。
(ert-deftest peta-journal-e2e-a-block-other-than-a-code-fence-closes-itself ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type ">>>")
    (peta-journal-e2e--keys "RET")
    (peta-journal-e2e--type "quoted")
    (should (equal (peta-journal--body-at (point)) ">>>\nquoted\n>>>"))))

;; HTMLの開始記号は、どの文字でも終わりうる。`<!--'は`-'で終わる。
(ert-deftest peta-journal-e2e-an-html-comment-closes-itself ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "<!--")
    (should (equal (peta-journal--body-at (point)) "<!--\n-->"))))

;; 続けて打った文字は、Enterのあとの項目に入る。
(ert-deftest peta-journal-e2e-keys-after-enter-go-to-the-new-item ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET b RET c"))
    (peta-journal--wait)
    (should (equal (peta-journal-e2e--shown) "• a\n• b\n• c\n"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("a" "b" "c"))))

;; TABで字下げし、バックタブで戻す。
(ert-deftest peta-journal-e2e-tab-changes-the-level ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "親")
    (peta-journal-e2e--keys "RET")
    (peta-journal-e2e--type "子")
    (peta-journal-e2e--keys "TAB")
    (should (= (plist-get (aref peta-journal--rows 1) :level) 2))
    (should (equal (peta-journal--body-at (point)) "子"))
    (peta-journal-e2e--wait-for
     "the level on the server"
     (lambda () (equal (mapcar (lambda (item) (plist-get item :level))
                               (peta-journal-e2e--saved-items))
                       '(1 2))))
    (peta-journal-e2e--keys "<backtab>")
    (should (= (plist-get (aref peta-journal--rows 1) :level) 1))))

;; 行頭のBackspaceで、上の項目につながる。
(ert-deftest peta-journal-e2e-backspace-joins-the-items ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a b RET c d"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("ab" "cd"))
    (peta-journal-e2e--keys "C-a DEL")
    (peta-journal-e2e--wait-for "the join"
                                (lambda () (equal (peta-journal-e2e--shown) "• abcd\n")))
    (peta-journal-e2e--wait-saved '("abcd"))))

;; 自動保存の前に行頭のBackspaceで結合しても、打ったばかりの文字はサーバーに残る。
(ert-deftest peta-journal-e2e-text-typed-before-a-join-is-saved ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a b RET"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("ab" ""))
    (peta-journal-e2e--type "cd")
    (peta-journal-e2e--keys "C-a DEL")
    (peta-journal-e2e--wait-for "the join"
                                (lambda () (equal (peta-journal-e2e--shown) "• abcd\n")))
    (peta-journal-e2e--wait-saved '("abcd"))))

;; 元に戻す操作はコアの履歴をたどる。打った文字を戻し、次に項目の分割を戻す。
(ert-deftest peta-journal-e2e-undo-goes-through-the-core ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a b RET c"))
    (peta-journal--wait)
    (should (equal (peta-journal-e2e--shown) "• ab\n• c\n"))
    (peta-journal-e2e--keys "C-/")
    (peta-journal-e2e--wait-for "the undo of the typing"
                                (lambda () (equal (peta-journal-e2e--shown) "• ab\n• \n")))
    (peta-journal-e2e--keys "C-/")
    (peta-journal-e2e--wait-for "the undo of the split"
                                (lambda () (= (length peta-journal--rows) 1)))))

;; タスクの状態が、バレットの後ろの記号に出る。
(ert-deftest peta-journal-e2e-the-task-cycle-shows-behind-the-bullet ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "仕事")
    (peta-journal-e2e--keys "C-c C-t")
    (peta-journal-e2e--wait-for "the mark of the task"
                                (lambda () (equal (peta-journal-e2e--shown) "• ☐ 仕事\n")))))

;; Markdownファイルの変更が、イベントストリームを通って画面に届く。
(ert-deftest peta-journal-e2e-a-change-in-the-file-reaches-the-buffer ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "前")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("前"))
    (let ((file (peta-journal-e2e--wait-for-file (expand-file-name "markdown" root) "前")))
      (with-temp-file file
        (set-buffer-file-coding-system 'utf-8-unix)
        (let ((coding-system-for-read 'utf-8))
          (insert-file-contents file))
        (goto-char (point-min))
        (search-forward "前")
        (replace-match "外からの変更")))
    (peta-journal-e2e--wait-for
     "the change in the file"
     (lambda () (string-search "外からの変更" (buffer-string))))))

;; ほかのクライアントがAPIで項目を変えると、開き直さなくてもバッファに出る。
(ert-deftest peta-journal-e2e-a-change-by-another-client-reaches-the-buffer ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "前")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("前"))
    ;; The request of the test names another client than the shell
    (let ((item (seq-first (peta-journal-e2e--saved-items))))
      (peta-journal-e2e--request "PUT" (concat "/api/items/" (plist-get item :id))
                                 '(:body "ほかのクライアントの変更")))
    (peta-journal-e2e--wait-for
     "the change by another client"
     (lambda () (string-search "ほかのクライアントの変更" (buffer-string))))))

;; 項目を消すと、画面からもサーバーからも消える。
(ert-deftest peta-journal-e2e-deleting-an-item ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET b"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("a" "b"))
    (peta-journal-e2e--keys "C-c C-d")
    (peta-journal-e2e--wait-for "the delete" (lambda () (= (length peta-journal--rows) 1)))
    (peta-journal-e2e--wait-saved '("a"))))

;; 選んだ項目は、まとめて動き、コピーされ、消える。消えると、選択は終わる。
(ert-deftest peta-journal-e2e-moving-copying-and-deleting-selected-items ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET b RET c"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("a" "b" "c"))
    (peta-journal-e2e--keys "C-p C-p C-c SPC n")
    (should peta-journal-selection-mode)
    (peta-journal-e2e--keys "M-S-<down>")
    (peta-journal-e2e--wait-saved '("c" "a" "b"))
    ;; The OS clipboard of the machine is left alone
    (let ((kill-ring nil)
          (interprogram-cut-function nil)
          (interprogram-paste-function nil))
      (peta-journal-e2e--keys "w")
      (peta-journal-e2e--wait-for "the copy" (lambda () kill-ring))
      (should (equal (car kill-ring) "- a\n- b")))
    (peta-journal-e2e--keys "d")
    (peta-journal-e2e--wait-saved '("c"))
    (peta-journal-e2e--wait-for "the end of the selection"
                                (lambda () (not peta-journal-selection-mode)))
    (should-not buffer-read-only)))

;; 項目の参照をコピーして別の項目に貼ると、その項目は元の項目を見せる。解除すると、ふつうの項目に戻る。
(ert-deftest peta-journal-e2e-pasting-and-removing-a-transclusion ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("a" ""))
    ;; The OS clipboard of the machine is left alone
    (let ((kill-ring nil)
          (interprogram-cut-function nil)
          (interprogram-paste-function nil))
      (peta-journal-e2e--keys "C-p C-c C-x w C-n C-c C-x t"))
    (peta-journal-e2e--wait-for
     "the transclusion"
     (lambda () (equal (plist-get (aref peta-journal--rows 1) :bullet) "transclusion")))
    (should (equal (plist-get (aref peta-journal--rows 1) :body) "a"))
    (peta-journal-e2e--keys "C-c C-x T")
    (peta-journal-e2e--wait-for
     "the plain item"
     (lambda () (equal (plist-get (aref peta-journal--rows 1) :bullet) "plain")))))

;; 項目をStashへ送ると、ページの最後の「# Stash」の見出しの下へ、子ごと移る。
(ert-deftest peta-journal-e2e-sending-an-item-to-the-stash ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET b TAB RET <backtab> c"))
    (peta-journal--wait)
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("a" "b" "c"))
    (peta-journal-e2e--keys "C-p C-p C-c C-x S")
    (peta-journal-e2e--wait-saved '("c" "# Stash" "a" "b"))
    (should (equal (mapcar (lambda (item) (plist-get item :level))
                           (peta-journal-e2e--saved-items))
                   '(1 1 2 3)))
    (should (equal (mapcar (lambda (row) (plist-get row :body)) peta-journal--rows)
                   '("c" "# Stash" "a" "b")))))

(defun peta-journal-e2e--open-page (path)
  "Ask the core to open the page at PATH, which is made when it does not exist.
No key does this: the tests use it to reach a page without the search box."
  (peta-journal--go "go_to_page" :path path))

(defun peta-journal-e2e--search-keys (query &rest keys)
  "Press C-c C-s, type QUERY without a pause, and press KEYS."
  (execute-kbd-macro (vconcat (kbd "C-c C-s") query (kbd (string-join keys " "))))
  (peta-journal--wait))

(defun peta-journal-e2e--wait-page-title (title)
  "Wait until the page shown has TITLE."
  (peta-journal-e2e--wait-for
   (format "the page %s, now %s" title peta-journal--title)
   (lambda () (equal peta-journal--title title))))

(defun peta-journal-e2e--rename (path)
  "Press C-c C-x r, replace the name it offers with PATH, and press RET."
  (execute-kbd-macro (vconcat (kbd "C-c C-x r C-a C-k") path (kbd "RET")))
  (peta-journal--wait))

;; `C-c C-x r`で名前を変えると、打ったばかりの文も新しい名前のファイルに書かれ、古いファイルは消える。
(ert-deftest peta-journal-e2e-renaming-moves-the-page-and-keeps-the-text ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "古い名前")
    (peta-journal-e2e--wait-page-title "古い名前")
    (peta-journal-e2e--type "本文")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("本文"))
    (let* ((markdown (expand-file-name "markdown" root))
           (old (peta-journal-e2e--wait-for-file markdown "本文")))
      (peta-journal-e2e--type "の続き")
      (peta-journal-e2e--rename "仕事/新しい名前")
      (peta-journal-e2e--wait-page-title "仕事/新しい名前")
      (should (equal peta-journal--page-path "仕事/新しい名前"))
      (should (equal (peta-journal-e2e--shown) "• 本文の続き\n"))
      (peta-journal-e2e--wait-saved '("本文の続き"))
      (peta-journal-e2e--wait-for
       "the file with the new name"
       (lambda ()
         (let ((file (peta-journal-e2e--file-with markdown "本文の続き")))
           (and file
                (string-suffix-p "仕事/新しい名前.md" file)
                (not (file-exists-p old))))))
      ;; 変えたあとも、そのまま書き続けられる
      (peta-journal-e2e--type "と結び")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("本文の続きと結び")))))

;; `C-c C-x d`で尋ねられて「はい」と答えると、ページのファイルが消え、今日のジャーナルが開く。
(ert-deftest peta-journal-e2e-deleting-the-page-removes-its-file-and-opens-today ()
  (peta-journal-e2e--with-app
    (let ((today peta-journal--page-id)
          (asked nil))
      (peta-journal-e2e--open-page "消すページ")
      (peta-journal-e2e--wait-page-title "消すページ")
      (peta-journal-e2e--type "消える本文")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("消える本文"))
      (let ((file (peta-journal-e2e--wait-for-file (expand-file-name "markdown" root) "消える本文")))
        (cl-letf (((symbol-function 'yes-or-no-p)
                   (lambda (prompt) (setq asked prompt) t)))
          (peta-journal-e2e--keys "C-c C-x d")
          (peta-journal-e2e--wait-for "the file to go" (lambda () (not (file-exists-p file))))
          (peta-journal-e2e--wait-for "today's journal"
                                      (lambda () (equal peta-journal--page-id today)))))
      (should (string-match-p "消すページ" asked)))))

;; サイドバーには、今のページのフォルダのページが並び、その行でRETを押すと、そのページが開く。
(ert-deftest peta-journal-e2e-the-sidebar-lists-the-folder-and-opens-its-pages ()
  (peta-journal-e2e--with-app
    (unwind-protect
        (progn
          (peta-journal-e2e--open-page "棚/一つ目")
          (peta-journal-e2e--wait-page-title "棚/一つ目")
          (peta-journal-e2e--open-page "棚/二つ目")
          (peta-journal-e2e--wait-page-title "棚/二つ目")
          (peta-journal-e2e--keys "C-c C-x s")
          (peta-journal-e2e--wait-for
           "the sidebar lists the folder"
           (lambda ()
             (with-current-buffer peta-journal--sidebar-buffer
               (save-excursion
                 (goto-char (point-min))
                 (re-search-forward "^  一つ目$" nil t)))))
          (with-current-buffer peta-journal--sidebar-buffer
            (goto-char (point-min))
            (re-search-forward "^  一つ目$")
            (beginning-of-line)
            (peta-journal-sidebar-open))
          (peta-journal-e2e--wait-page-title "棚/一つ目"))
      (when-let* ((window (get-buffer-window peta-journal--sidebar-buffer)))
        (delete-window window)))))

;; フォルダのないパスを付けると、ページはフォルダの外へ出る。
(ert-deftest peta-journal-e2e-a-page-can-move-out-of-its-folder ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "フォルダ/メモ")
    (peta-journal-e2e--wait-page-title "フォルダ/メモ")
    (peta-journal-e2e--rename "メモ")
    (peta-journal-e2e--wait-page-title "メモ")
    (should (equal peta-journal--page-path "メモ"))))

;; indexページで`C-c C-x r`を押すと、フォルダの名前が変わり、indexページはそのフォルダへ移る。
(ert-deftest peta-journal-e2e-renaming-an-index-page-renames-its-folder ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "古いフォルダ/index")
    (peta-journal-e2e--wait-for "the index page"
                                (lambda () (equal peta-journal--index-folder "古いフォルダ")))
    (peta-journal-e2e--type "索引")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("索引"))
    (peta-journal-e2e--rename "新しいフォルダ")
    (peta-journal-e2e--wait-for "the new folder"
                                (lambda () (equal peta-journal--index-folder "新しいフォルダ")))
    (let ((markdown (expand-file-name "markdown" root)))
      (peta-journal-e2e--wait-for
       "the file in the new folder"
       (lambda ()
         (let ((file (peta-journal-e2e--file-with markdown "索引")))
           (and file (string-suffix-p "新しいフォルダ/index.md" file))))))))

;; 下にページのないindexページは、`C-c C-x D`で「はい」と答えると、フォルダの名前のページになる。
(ert-deftest peta-journal-e2e-demoting-an-index-page-makes-it-the-page-of-its-folder ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "上/降格/index")
    (peta-journal-e2e--wait-for "an index page that can be demoted"
                                (lambda () (equal peta-journal--demote-name "降格")))
    (peta-journal-e2e--type "本文")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("本文"))
    (cl-letf (((symbol-function 'yes-or-no-p) (lambda (_prompt) t)))
      (peta-journal-e2e--keys "C-c C-x D")
      (peta-journal-e2e--wait-for "the page of the folder"
                                  (lambda () (equal peta-journal--page-path "上/降格"))))
    (should-not peta-journal--index-folder)
    (peta-journal-e2e--wait-saved '("本文"))))

;; ジャーナルでは、名前を尋ねずに断る。
(ert-deftest peta-journal-e2e-a-journal-refuses-the-rename ()
  (peta-journal-e2e--with-app
    (should-error (execute-kbd-macro (kbd "C-c C-x r")) :type 'user-error)))

(defun peta-journal-e2e--write-image (file bytes)
  "Write the unibyte string BYTES to FILE, as they are."
  (let ((coding-system-for-write 'no-conversion))
    (write-region bytes nil file nil 'silent)))

(defun peta-journal-e2e--bytes-of (file)
  "The bytes in FILE, as a unibyte string."
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (let ((coding-system-for-read 'no-conversion))
      (insert-file-contents-literally file))
    (buffer-string)))

(defun peta-journal-e2e--image-link (original bytes root)
  "The link to the image that came as ORIGINAL, shown between 前 and と後.
Its text is ORIGINAL.  It points to the name the server made: an HMTID and
the extension.  That file must hold BYTES in the notes under ROOT."
  (let ((shown (peta-journal-e2e--shown)))
    (should (string-match
             (concat "\\`• 前\\(!\\[" (regexp-quote original) "\\]"
                     "(\\.\\./assets/\\([0-9]\\{14\\}_[0-9A-Z]\\{7\\}\\.png\\))\\)と後\n\\'")
             shown))
    (let ((link (match-string 1 shown))
          (saved (match-string 2 shown)))
      (should (equal (peta-journal-e2e--bytes-of
                      (expand-file-name saved (expand-file-name "markdown/assets" root)))
                     bytes))
      link)))

;; `C-c C-x i`で選んだ画像は、中身のままサーバーへ届き、カーソル位置にリンクが入る。続けて打った文字はリンクの後ろに入る。
;; リンクの文字は元の名前で、リンク先はサーバーが付けた名前である。
(ert-deftest peta-journal-e2e-an-inserted-image-is-uploaded-and-linked ()
  (peta-journal-e2e--with-app
    ;; Bytes that break when read as text: a line end, a zero and a high byte
    (let ((bytes (unibyte-string #x89 ?P ?N ?G 13 10 0 255))
          (image (expand-file-name "pic.png" root)))
      (peta-journal-e2e--write-image image bytes)
      (peta-journal-e2e--type "前後")
      (peta-journal-e2e--keys "C-b")
      (execute-kbd-macro (vconcat (kbd "C-c C-x i C-a C-k") image (kbd "RET")))
      (peta-journal--wait)
      (peta-journal-e2e--type "と")
      (let ((link (peta-journal-e2e--image-link "pic.png" bytes root)))
        (peta-journal-e2e--keys "C-x C-s")
        (peta-journal-e2e--wait-saved (list (format "前%sと後" link)))))))

;; クリップボードに画像だけがあるとき、`C-y`でその画像が中身のままサーバーへ届き、カーソル位置にリンクが入る。
;; The clipboard is replaced: the real one belongs to the person at the machine.
(ert-deftest peta-journal-e2e-an-image-in-the-clipboard-is-pasted ()
  (peta-journal-e2e--with-app
    (let ((bytes (unibyte-string #x89 ?P ?N ?G 13 10 0 255)))
      (peta-journal-e2e--type "前後")
      (peta-journal-e2e--keys "C-b")
      (cl-letf (((symbol-function 'gui-get-selection)
                 (lambda (_selection type)
                   (pcase type
                     ('TARGETS [BITMAP image/png])
                     ('image/png bytes)))))
        (peta-journal-e2e--keys "C-y"))
      (peta-journal-e2e--type "と")
      (let ((link (peta-journal-e2e--image-link "image.png" bytes root)))
        (peta-journal-e2e--keys "C-x C-s")
        (peta-journal-e2e--wait-saved (list (format "前%sと後" link)))))))

;; 読めないファイルを選ぶと、その理由を伝え、本文は変わらない。そのまま書き続けられる。
(ert-deftest peta-journal-e2e-a-file-that-cannot-be-read-is-reported ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "本文")
    (peta-journal-insert-image (expand-file-name "no-such-file.png" root))
    (should (equal (peta-journal-e2e--shown) "• 本文\n"))
    (with-current-buffer "*Messages*"
      (goto-char (point-max))
      (should (search-backward "Could not insert the image" nil t)))
    (peta-journal-e2e--type "の続き")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("本文の続き"))))

(defun peta-journal-e2e--shown-image ()
  "The image shown in the buffer, or nil."
  (seq-some (lambda (overlay) (overlay-get overlay 'display))
            (overlays-in (point-min) (point-max))))

;; 挿入した画像は、カーソルが項目を出ると、サーバーから取得した中身のまま見える。
;; バッチ実行では描けないので、描くはずの画像の中身までを確かめる。PNGを表示できないEmacsでは飛ばす。
(ert-deftest peta-journal-e2e-an-inserted-image-is-shown-after-the-point-leaves ()
  ;; Such a build shows the link as text, which the tests of the mode check
  (skip-unless (image-type-available-p 'png))
  (cl-letf (((symbol-function 'display-images-p) (lambda (&rest _) t)))
    (peta-journal-e2e--with-app
      ;; Bytes that break when read as text: a line end, a zero and a high byte
      (let ((bytes (unibyte-string #x89 ?P ?N ?G 13 10 26 10 0 255 13 10 13 10))
            (image (expand-file-name "shown.png" root)))
        (peta-journal-e2e--write-image image bytes)
        (execute-kbd-macro (vconcat (kbd "C-c C-x i C-a C-k") image (kbd "RET")))
        (peta-journal--wait)
        (should-not (peta-journal-e2e--shown-image))
        (peta-journal-e2e--keys "RET")
        (peta-journal-e2e--wait-for "the image" #'peta-journal-e2e--shown-image)
        (should (equal (image-property (peta-journal-e2e--shown-image) :data) bytes))))))

;; サーバーにないファイルは、取得できなかったものとして扱う。
(ert-deftest peta-journal-e2e-a-file-not-on-the-server-is-missing ()
  (peta-journal-e2e--with-app
    (let ((url (peta-journal--image-url (list :path "assets/no-such-file.png"))))
      (peta-journal--fetch-image url)
      (peta-journal-e2e--wait-for "the answer of the server"
                                  (lambda () (gethash url peta-journal--images)))
      (should (eq (gethash url peta-journal--images) 'missing)))))

(defun peta-journal-e2e--page-id ()
  "The id of the page in the buffer of the app.
The list of properties takes the point, so the current buffer may not be the page."
  (buffer-local-value 'peta-journal--page-id (get-buffer "*PetaJournal*")))

(defun peta-journal-e2e--saved-tags ()
  "The tags of the page on screen, as the server has them, in a list."
  (let* ((answer (peta-journal-e2e--request "GET" (concat "/api/pages/" (peta-journal-e2e--page-id))))
         (properties (plist-get (plist-get answer :page) :properties)))
    (append (plist-get properties :tags) nil)))

(defun peta-journal-e2e--wait-saved-tags (tags)
  "Wait until the server has exactly TAGS on the page on screen."
  (peta-journal-e2e--wait-for
   (format "the server to have the tags %S, has %S" tags (peta-journal-e2e--saved-tags))
   (lambda () (equal (peta-journal-e2e--saved-tags) tags))))

(defun peta-journal-e2e--properties-text ()
  "What the list of properties shows."
  (with-current-buffer "*PetaJournal Properties*"
    (buffer-substring-no-properties (point-min) (point-max))))

;; `C-c C-x p`で開いた一覧の中で`s`を押してプロパティを設定すると、サーバーに保存され、一覧に出る。
;; もう一度設定すると今の値から直せて、`d`で消せる。`q`で一覧を閉じると、打ちかけの文はそのまま残っている。
(ert-deftest peta-journal-e2e-a-property-is-set-changed-and-removed ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "プロパティのページ")
    (peta-journal-e2e--wait-page-title "プロパティのページ")
    (peta-journal-e2e--type "打ちかけ")
    (execute-kbd-macro (vconcat (kbd "C-c C-x p s") "tags" (kbd "RET") "emacs, memo" (kbd "RET")))
    (peta-journal-e2e--wait-saved-tags '("emacs" "memo"))
    (peta-journal-e2e--wait-for
     "the tags in the list"
     (lambda () (string-search "tags" (peta-journal-e2e--properties-text))))
    (should (string-search "emacs, memo" (peta-journal-e2e--properties-text)))

    (execute-kbd-macro (vconcat (kbd "s") "tags" (kbd "RET") (kbd "C-e") ", more" (kbd "RET")))
    (peta-journal-e2e--wait-saved-tags '("emacs" "memo" "more"))

    (execute-kbd-macro (vconcat (kbd "d") "tags" (kbd "RET")))
    (peta-journal-e2e--wait-saved-tags nil)
    (peta-journal-e2e--wait-for
     "the tags to leave the list"
     (lambda () (not (string-search "tags" (peta-journal-e2e--properties-text)))))
    (peta-journal-e2e--keys "q")
    (should (equal (peta-journal--body-at (point)) "打ちかけ"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("打ちかけ"))))

;; 作成日時を一覧の`s`で直すと、サーバーに保存され、一覧にはローカル時刻で出る。
(ert-deftest peta-journal-e2e-a-date-is-set-in-local-time ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "日時のページ")
    (peta-journal-e2e--wait-page-title "日時のページ")
    (execute-kbd-macro
     (vconcat (kbd "C-c C-x p s") "created" (kbd "RET C-a C-k") "2026-01-02 03:04" (kbd "RET")))
    (peta-journal-e2e--wait-for
     "the new date in the list"
     (lambda () (string-search "2026-01-02 03:04" (peta-journal-e2e--properties-text))))
    (let ((saved (plist-get (plist-get (peta-journal-e2e--request
                                        "GET" (concat "/api/pages/" (peta-journal-e2e--page-id)))
                                       :page)
                            :createdDate)))
      (should (equal (format-time-string "%Y-%m-%d %H:%M" (/ saved 1000))
                     "2026-01-02 03:04")))))

;; 日時のキーを新しく付けるときは、現在時刻が入っているので、Enterだけで保存される。
(ert-deftest peta-journal-e2e-a-new-date-starts-at-now ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "新しい日時のページ")
    (peta-journal-e2e--wait-page-title "新しい日時のページ")
    (peta-journal-e2e--wait-for
     "the key published" (lambda () (seq-find (lambda (k) (equal (plist-get k :key) "published"))
                                              peta-journal--free-keys)))
    (let ((before (format-time-string "%Y-%m-%d %H:%M")))
      (execute-kbd-macro (vconcat (kbd "C-c C-x p s") "published" (kbd "RET RET")))
      (peta-journal-e2e--wait-for
       "the date in the list"
       (lambda () (string-search "published" (peta-journal-e2e--properties-text))))
      (let ((after (format-time-string "%Y-%m-%d %H:%M"))
            (shown (peta-journal-e2e--properties-text)))
        (should (or (string-search (concat "published (datetime)  " before) shown)
                    (string-search (concat "published (datetime)  " after) shown)))))))

;; ジャーナルでは、プロパティの一覧を開かずに断る。
(ert-deftest peta-journal-e2e-a-journal-refuses-a-property ()
  (peta-journal-e2e--with-app
    (should-error (execute-kbd-macro (kbd "C-c C-x p")) :type 'user-error)))

;; `C-c C-s`で検索語を打って`RET`を押すと、その語の新しいページができて開く。間を置かなくてもよい。
(ert-deftest peta-journal-e2e-the-search-box-creates-a-page ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--search-keys "新しいページ" "RET")
    (peta-journal-e2e--wait-page-title "新しいページ")
    (should-not (get-buffer "*PetaJournal Search*"))
    (should (equal (buffer-name (window-buffer (selected-window))) "*PetaJournal*"))))

;; 項目の検索結果を C-n で選んで開くと、そのページへ移る。ページ名に当たらない語でも見つかる。
(ert-deftest peta-journal-e2e-the-search-box-finds-an-item ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "探すことば")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("探すことば"))
    (let ((journal peta-journal--page-id))
      (peta-journal-e2e--open-page "他のページ")
      (peta-journal-e2e--wait-page-title "他のページ")
      ;; The first result is the entry that creates a page with this name
      (peta-journal-e2e--search-keys "探すことば" "C-n" "RET")
      (peta-journal-e2e--wait-for "the page of the item"
                                  (lambda () (equal peta-journal--page-id journal))))))

;; 何も打たずに`TAB`を押すと最近のページが並び、`RET`で先頭のページが開く。
(ert-deftest peta-journal-e2e-the-search-box-opens-a-recent-page ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "最近のページ")
    (peta-journal-e2e--wait-page-title "最近のページ")
    (peta-journal-e2e--type "本文")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("本文"))
    (peta-journal-e2e--wait-for "the page among the recent pages"
                                (lambda () (equal (car peta-journal--recent) "最近のページ")))
    (peta-journal-e2e--keys "C-c C-j")
    (peta-journal-e2e--wait-for "today's journal"
                                (lambda () (not (equal peta-journal--title "最近のページ"))))
    (peta-journal-e2e--search-keys "" "TAB" "RET")
    (peta-journal-e2e--wait-page-title "最近のページ")
    (should-not (get-buffer "*PetaJournal Search*"))))

;; C-g で閉じると、どこへも移らず、結果の窓も消える。
(ert-deftest peta-journal-e2e-quitting-the-search-box-stays-on-the-page ()
  (peta-journal-e2e--with-app
    (let ((journal peta-journal--page-id))
      (condition-case nil
          (peta-journal-e2e--search-keys "途中で" "C-g")
        (quit nil))
      (peta-journal--wait)
      (should-not (get-buffer "*PetaJournal Search*"))
      (should (equal peta-journal--page-id journal))
      (should-not peta-journal--searching))))

;; リンクを開くと、そのページへ移り、バックリンクの一覧から元のページへ戻れる。
(ert-deftest peta-journal-e2e-a-link-opens-and-its-backlink-leads-back ()
  (peta-journal-e2e--with-app
    (let ((journal peta-journal--page-id))
      (peta-journal-e2e--type "[[リンク先")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("[[リンク先]]"))
      (peta-journal-e2e--keys "C-c C-o")
      (peta-journal-e2e--wait-for "the linked page"
                                  (lambda () (not (equal peta-journal--page-id journal))))
      (peta-journal-e2e--wait-for "the backlinks"
                                  (lambda () (> (length peta-journal--backlinks) 0)))
      (peta-journal-backlinks)
      (with-current-buffer "*PetaJournal Backlinks*"
        (should (string-search "[[リンク先]]" (buffer-string)))
        (select-window (get-buffer-window (current-buffer)))
        (peta-journal-list-open))
      (with-current-buffer "*PetaJournal*"
        (peta-journal-e2e--wait-for "the page with the link"
                                    (lambda () (equal peta-journal--page-id journal)))))))

;; リンクで開いたページから`C-c C-b`で戻り、`C-c C-x f`でまたそのページへ進める。
(ert-deftest peta-journal-e2e-back-and-forward-follow-the-pages-opened ()
  (peta-journal-e2e--with-app
    (let ((journal peta-journal--page-id))
      (peta-journal-e2e--type "[[進む先")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("[[進む先]]"))
      (peta-journal-e2e--keys "C-c C-o")
      (peta-journal-e2e--wait-page-title "進む先")
      (peta-journal-e2e--keys "C-c C-b")
      (peta-journal-e2e--wait-for "the journal again"
                                  (lambda () (equal peta-journal--page-id journal)))
      (peta-journal-e2e--keys "C-c C-x f")
      (peta-journal-e2e--wait-page-title "進む先"))))

;; `[[`の中で`C-M-i`を押すと、打った文字に合うページの名前が入り、保存される。
(ert-deftest peta-journal-e2e-a-wiki-link-is-completed-with-a-page-name ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--open-page "補完の行き先")
    (peta-journal-e2e--wait-page-title "補完の行き先")
    (peta-journal-e2e--keys "C-c C-j")
    (peta-journal-e2e--wait-for "today's journal"
                                (lambda () (not (equal peta-journal--title "補完の行き先"))))
    (peta-journal-e2e--type "see [[補完")
    (peta-journal-e2e--keys "C-M-i")
    (should (equal (peta-journal--body-at (point)) "see [[補完の行き先]]"))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("see [[補完の行き先]]"))))

;; 次のジャーナルへ移り、前のページへ戻れる。
(ert-deftest peta-journal-e2e-journals-can-be-walked ()
  (peta-journal-e2e--with-app
    (let ((today peta-journal--page-id))
      (peta-journal-e2e--keys "C-c C-n")
      (peta-journal-e2e--wait-for "the next journal"
                                  (lambda () (not (equal peta-journal--page-id today))))
      (peta-journal-e2e--keys "C-c C-b")
      (peta-journal-e2e--wait-for "the journal before"
                                  (lambda () (equal peta-journal--page-id today))))))

(defun peta-journal-e2e--pick-in-calendar (key title)
  "Open the calendar, press KEY on 2026-02-10, and wait for the page with TITLE."
  (let ((page (current-buffer)))
    (peta-journal-e2e--keys "C-c C-x c")
    (calendar-goto-date '(2 10 2026))
    (execute-kbd-macro (kbd key))
    (with-current-buffer page
      (peta-journal-e2e--wait-for (format "the page %s" title)
                                  (lambda () (equal peta-journal--title title))))))

;; カレンダーで選んだ日のジャーナルが開く。wは、その日の週のページを開く。
(ert-deftest peta-journal-e2e-the-calendar-opens-the-journal-of-a-day-and-of-its-week ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--pick-in-calendar "RET" "2026年2月10日 火曜日")
    (peta-journal-e2e--pick-in-calendar "w" "2026 Week 07")))

;; Emacsを閉じても、打ったばかりの文字は保存される。
(ert-deftest peta-journal-e2e-stopping-saves-what-was-typed ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "閉じる直前")
    (let ((page peta-journal--page-id))
      (peta-journal-stop)
      (should-not (process-live-p peta-journal--process))
      (let ((items (plist-get (peta-journal-e2e--request "GET" (concat "/api/pages/" page))
                              :items)))
        (should (equal (mapcar (lambda (item) (plist-get item :body)) items)
                       '("閉じる直前")))))))

(defun peta-journal-e2e--open-in-new-buffer (path)
  "Open the page at PATH in a new buffer, and return the buffer on screen."
  (peta-journal-new-buffer path)
  (window-buffer (selected-window)))

;; 別々のページを2つのバッファで編集でき、どちらも保存される。
(ert-deftest peta-journal-e2e-two-buffers-edit-two-pages ()
  (peta-journal-e2e--with-app
    (let ((first (current-buffer))
          (journal peta-journal--page-id)
          (second (peta-journal-e2e--open-in-new-buffer "E2E/二つ目")))
      (should-not (eq second first))
      (with-current-buffer second
        (should peta-journal--page-id)
        (should-not (equal peta-journal--page-id journal))
        (peta-journal-e2e--type "二つ目の本文")
        (peta-journal-e2e--keys "C-x C-s")
        (peta-journal-e2e--wait-saved '("二つ目の本文")))
      (set-window-buffer (selected-window) first)
      (with-current-buffer first
        (should (equal peta-journal--page-id journal))
        (peta-journal-e2e--type "一つ目の本文")
        (peta-journal-e2e--keys "C-x C-s")
        (peta-journal-e2e--wait-saved '("一つ目の本文")))
      (with-current-buffer second
        (should (equal (peta-journal-e2e--shown) "• 二つ目の本文\n"))))))

;; すでに開いているページを新しいバッファで開こうとすると、開いているバッファへ移る。
(ert-deftest peta-journal-e2e-a-page-already-open-is-not-opened-twice ()
  (peta-journal-e2e--with-app
    (let ((second (peta-journal-e2e--open-in-new-buffer "E2E/二つ目")))
      (should (eq (peta-journal-e2e--open-in-new-buffer "E2E/二つ目") second))
      (should (= (length (peta-journal--pages)) 2)))))

;; 片方のバッファを閉じても、もう片方のシェルは動き続け、編集できる。
(ert-deftest peta-journal-e2e-closing-one-buffer-leaves-the-other-working ()
  (peta-journal-e2e--with-app
    (let ((first (current-buffer))
          (second (peta-journal-e2e--open-in-new-buffer "E2E/二つ目")))
      (kill-buffer second)
      (set-window-buffer (selected-window) first)
      (with-current-buffer first
        (should (process-live-p peta-journal--process))
        (should peta-journal--idle-timer)
        (peta-journal-e2e--type "残った本文")
        (peta-journal-e2e--keys "C-x C-s")
        (peta-journal-e2e--wait-saved '("残った本文"))))))

;; 検索は、検索を始めたバッファのページだけを移す。
(ert-deftest peta-journal-e2e-the-search-box-moves-only-its-own-buffer ()
  (peta-journal-e2e--with-app
    (let ((first (current-buffer))
          (journal peta-journal--page-id)
          (second (peta-journal-e2e--open-in-new-buffer "E2E/二つ目")))
      (with-current-buffer second
        (peta-journal-e2e--search-keys "検索先のページ" "RET")
        (peta-journal-e2e--wait-page-title "検索先のページ"))
      (should (eq (window-buffer (selected-window)) second))
      (should (equal (buffer-local-value 'peta-journal--page-id first) journal)))))

(provide 'peta-journal-e2e)

;;; peta-journal-e2e.el ends here
