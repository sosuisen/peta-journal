;;; peta-journal-e2e.el --- Emacs, the shell and a real server together -*- lexical-binding: t; -*-

;;; Commentary:

;; Starts a real journal server in a temporary folder and drives the real shell with real keys.
;; It runs what the other tests cannot: runtime.rs, the JSON lines and the save to the server.
;;
;; Build both programs first, then run from crates/shell-emacs:
;;   emacs -Q --batch -L lisp -l lisp/test/peta-journal-e2e.el -f ert-run-tests-batch-and-exit
;;
;; PETA_E2E_SERVER is the journal-server program.  The tests are skipped when it is missing.
;;
;; See ADR-002: the server gets its own config folder.  Without it, it would export into the real
;; notes of the user.  The tests also stop when the server reports another markdown folder.

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
  (let ((target (expand-file-name "../../../../target/debug/" peta-journal-e2e--here)))
    (seq-find #'file-executable-p
              (list (expand-file-name "peta-shell-emacs.exe" target)
                    (expand-file-name "peta-shell-emacs" target)))))

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
       (format "%s %s HTTP/1.0\r\nContent-Type: application/json\r\nContent-Length: %d\r\n\r\n%s"
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

(defun peta-journal-e2e--write-config (root markdown)
  "Write the config of the server in ROOT, with its notes in MARKDOWN.
Return its folder."
  (let ((config (expand-file-name "config" root)))
    (make-directory config t)
    (make-directory markdown t)
    (with-temp-file (expand-file-name "config.json" config)
      (insert (json-serialize (list :markdownDir markdown :weekNumbering "iso"))))
    config))

(defun peta-journal-e2e--start-server (root)
  "Start the test server with its files in ROOT, and return its process.
Signals an error when its notes would not stay inside ROOT."
  (let* ((markdown (expand-file-name "markdown" root))
         (config (peta-journal-e2e--write-config root markdown))
         (process
          (make-process
           :name "peta-e2e-server"
           :buffer (get-buffer-create " *peta-e2e-server*")
           :noquery t
           :command (list (peta-journal-e2e--server-program)
                          "--port" (number-to-string peta-journal-e2e--port)
                          "--data-dir" (expand-file-name "data" root)
                          "--index-dir" (expand-file-name "index" root)
                          "--config-dir" config))))
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
         (when (get-buffer "*PetaJournal*")
           (kill-buffer "*PetaJournal*"))
         (peta-journal-stop)
         (peta-journal-e2e--stop-server server root)))))

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
                (insert-file-contents file)
                (search-forward text nil t)))
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

;; Enterで項目が分かれ、カーソルは新しい項目の先頭へ移る。
(ert-deftest peta-journal-e2e-enter-splits-the-item ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "abcd")
    (peta-journal-e2e--keys "C-b C-b RET")
    (should (equal (buffer-string) "• ab\n• cd\n"))
    (should (equal (peta-journal--body-at (point)) "cd"))
    (should (= (peta-journal--cursor) 0))
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("ab" "cd"))))

;; 続けて打った文字は、Enterのあとの項目に入る。
(ert-deftest peta-journal-e2e-keys-after-enter-go-to-the-new-item ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a RET b RET c"))
    (peta-journal--wait)
    (should (equal (buffer-string) "• a\n• b\n• c\n"))
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
                                (lambda () (equal (buffer-string) "• abcd\n")))
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
                                (lambda () (equal (buffer-string) "• abcd\n")))
    (peta-journal-e2e--wait-saved '("abcd"))))

;; 元に戻す操作はコアの履歴をたどる。打った文字を戻し、次に項目の分割を戻す。
(ert-deftest peta-journal-e2e-undo-goes-through-the-core ()
  (peta-journal-e2e--with-app
    (execute-kbd-macro (kbd "a b RET c"))
    (peta-journal--wait)
    (should (equal (buffer-string) "• ab\n• c\n"))
    (peta-journal-e2e--keys "C-/")
    (peta-journal-e2e--wait-for "the undo of the typing"
                                (lambda () (equal (buffer-string) "• ab\n• \n")))
    (peta-journal-e2e--keys "C-/")
    (peta-journal-e2e--wait-for "the undo of the split"
                                (lambda () (= (length peta-journal--rows) 1)))))

;; タスクの状態が、バレットに出る。
(ert-deftest peta-journal-e2e-the-task-cycle-shows-on-the-bullet ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "仕事")
    (peta-journal-e2e--keys "C-c C-t")
    (peta-journal-e2e--wait-for "the task bullet"
                                (lambda () (string-prefix-p "☐" (buffer-string))))))

;; Markdownファイルの変更が、イベントストリームを通って画面に届く。
(ert-deftest peta-journal-e2e-a-change-in-the-file-reaches-the-buffer ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "前")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("前"))
    (let ((file (peta-journal-e2e--wait-for-file (expand-file-name "markdown" root) "前")))
      (with-temp-file file
        (set-buffer-file-coding-system 'utf-8-unix)
        (insert-file-contents file)
        (goto-char (point-min))
        (search-forward "前")
        (replace-match "外からの変更")))
    (peta-journal-e2e--wait-for
     "the change in the file"
     (lambda () (string-search "外からの変更" (buffer-string))))))

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

;; 検索の結果を開くと、そのページへ移る。
(ert-deftest peta-journal-e2e-searching-finds-an-item ()
  (peta-journal-e2e--with-app
    (peta-journal-e2e--type "探すことば")
    (peta-journal-e2e--keys "C-x C-s")
    (peta-journal-e2e--wait-saved '("探すことば"))
    (peta-journal-search "探すことば")
    (peta-journal-e2e--wait-for
     "a search result"
     (lambda ()
       (with-current-buffer "*PetaJournal Search*"
         (string-search "探すことば" (buffer-substring (min (point-max) 20) (point-max))))))
    (set-window-buffer (selected-window) "*PetaJournal*")))

;; ページの移動は、名前で開いて、その本文が見える。
(ert-deftest peta-journal-e2e-finding-a-page-opens-it ()
  (peta-journal-e2e--with-app
    (let ((journal peta-journal--page-id))
      (peta-journal-find-page "E2E/メモ")
      (peta-journal-e2e--wait-for "the other page"
                                  (lambda () (not (equal peta-journal--page-id journal))))
      (peta-journal-e2e--type "ページの本文")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("ページの本文")))))

;; リンクを開くと、そのページへ移り、バックリンクの一覧から元のページへ戻れる。
(ert-deftest peta-journal-e2e-a-link-opens-and-its-backlink-leads-back ()
  (peta-journal-e2e--with-app
    (let ((journal peta-journal--page-id))
      (peta-journal-e2e--type "[[リンク先]]")
      (peta-journal-e2e--keys "C-x C-s")
      (peta-journal-e2e--wait-saved '("[[リンク先]]"))
      (peta-journal-e2e--keys "C-b C-b C-c C-o")
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

;; 次のジャーナルへ移り、前のページへ戻れる。
(ert-deftest peta-journal-e2e-journals-can-be-walked ()
  (peta-journal-e2e--with-app
    (let ((today peta-journal--page-id))
      (peta-journal-e2e--keys "C-c C-n")
      (peta-journal-e2e--wait-for "the next journal"
                                  (lambda () (not (equal peta-journal--page-id today))))
      (peta-journal-e2e--keys "C-c C-l")
      (peta-journal-e2e--wait-for "the journal before"
                                  (lambda () (equal peta-journal--page-id today))))))

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

(provide 'peta-journal-e2e)

;;; peta-journal-e2e.el ends here
