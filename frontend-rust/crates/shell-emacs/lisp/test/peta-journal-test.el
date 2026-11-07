;;; peta-journal-test.el --- Tests of the buffer side of peta-journal -*- lexical-binding: t; -*-

;;; Commentary:

;; These tests run without a process: what would go to the shell is collected in a list.
;; Run them with:
;;   emacs -Q --batch -L lisp -l lisp/test/peta-journal-test.el -f ert-run-tests-batch-and-exit

;;; Code:

(require 'ert)
(require 'cl-lib)
(require 'peta-journal)

(defvar peta-journal-test--sent nil
  "The lines a test sent to the shell, oldest first.")

(defun peta-journal-test--row (id level body &rest more)
  "A row as the shell sends it, with ID, LEVEL and BODY, and MORE fields."
  (append more
          (list :id id :level level :body body :bullet "plain" :collapsed nil :header nil)))

(defmacro peta-journal-test--with-rows (rows &rest body)
  "Run BODY in a peta-journal buffer that shows ROWS, with the shell replaced."
  (declare (indent 1))
  `(with-temp-buffer
     (rename-buffer "*PetaJournal*")
     (peta-journal-mode)
     (setq peta-journal-test--sent nil)
     (let ((peta-journal--seq 0))
       (cl-letf (((symbol-function 'peta-journal--send)
                  (lambda (type &rest fields)
                    (setq peta-journal-test--sent
                          (append peta-journal-test--sent (list (cons type fields))))))
                 ((symbol-function 'peta-journal--wait) #'ignore))
         (peta-journal--apply-rows (vconcat ,rows) "page" 0)
         ,@body))))

(defun peta-journal-test--goto (id cursor)
  "Put the point in the item ID at CURSOR, as a command would."
  (peta-journal--goto-item id cursor)
  (peta-journal--post-command))

(defun peta-journal-test--type (text)
  "Type TEXT at the point, key by key."
  (dolist (key (string-to-list text))
    (let ((last-command-event key))
      (self-insert-command 1))))

(defun peta-journal-test--sent-of (type)
  "The lines of TYPE sent so far."
  (cl-remove-if-not (lambda (line) (equal (car line) type)) peta-journal-test--sent))

;;;; Drawing

;; 各項目は、バレットと本文の1行として描かれる。バッファの文字に字下げは入らない。
(ert-deftest peta-journal-a-row-is-a-bullet-and-its-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "親")
                                      (peta-journal-test--row "b" 2 "子"))
    (should (equal (buffer-string) "• 親\n• 子\n"))))

;; 字下げは表示だけで、レベルの分の幅になる。
(ert-deftest peta-journal-the-indent-of-a-row-follows-its-level ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "親")
                                      (peta-journal-test--row "b" 3 "孫"))
    (peta-journal--goto-item "b" 0)
    (should (equal (get-text-property (line-beginning-position) 'line-prefix) "    "))))

;; タスクの状態は、バレットの形で示される。
(ert-deftest peta-journal-the-bullet-shows-the-task-state ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x" :bullet "task")
            (peta-journal-test--row "b" 1 "y" :bullet "running")
            (peta-journal-test--row "c" 1 "z" :bullet "done"))
    (should (equal (buffer-string) "☐ x\n◐ y\n☑ z\n"))))

;; ジャーナルの日付は、項目の上の編集できない行である。
(ert-deftest peta-journal-a-date-header-is-a-line-above-its-row ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "今日")
            (peta-journal-test--row "b" 1 "昨日" :header "2026-11-05"))
    (should (equal (buffer-string) "• 今日\n── 2026-11-05 ──\n• 昨日\n"))
    (goto-char (point-min))
    (search-forward "──")
    (should-error (insert "x") :type 'text-read-only)))

;; 変わった行だけが書き換えられ、他の行の位置は動かない。
(ert-deftest peta-journal-only-the-changed-rows-are-rewritten ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal--goto-item "a" 0)
    (let ((marker (point-marker)))
      (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "x")
                                        (peta-journal-test--row "b" 1 "yy"))
                                "page" 0)
      (should (equal (buffer-string) "• x\n• yy\n"))
      (should (= (marker-position marker) (peta-journal--body-start (point-min)))))))

;; 行が書き換えられても、カーソルは同じ項目の同じ文字の位置に残る。
(ert-deftest peta-journal-the-point-stays-in-its-item-when-rows-change ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "abc"))
    (peta-journal--goto-item "b" 2)
    (peta-journal--apply-rows (vector (peta-journal-test--row "z" 1 "新")
                                      (peta-journal-test--row "a" 2 "x")
                                      (peta-journal-test--row "b" 2 "abc"))
                              "page" 0)
    (should (equal (peta-journal--id-at (point)) "b"))
    (should (= (peta-journal--cursor) 2))))

;; 項目が消えると、カーソルは同じ順番の項目へ移る。
(ert-deftest peta-journal-the-point-moves-to-the-same-place-when-its-item-goes ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y")
                                      (peta-journal-test--row "c" 1 "z"))
    (peta-journal--goto-item "b" 0)
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "c" 1 "z"))
                              "page" 0)
    (should (equal (peta-journal--id-at (point)) "c"))))

;; 別のページが開くと、カーソルは先頭の項目へ移る。
(ert-deftest peta-journal-a-new-page-puts-the-point-on-its-first-row ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal--goto-item "b" 1)
    (peta-journal--apply-rows (vector (peta-journal-test--row "p" 1 "別")
                                      (peta-journal-test--row "q" 1 "頁"))
                              "other" 0)
    (should (equal (peta-journal--id-at (point)) "p"))
    (should (= (peta-journal--cursor) 0))))

;; 複数行の本文は、次の項目の手前までが1つの項目である。
(ert-deftest peta-journal-a-body-of-several-lines-is-one-item ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "一\n- 二\n")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal--goto-item "a" 3)
    (should (equal (peta-journal--id-at (point)) "a"))
    (should (equal (peta-journal--body-at (point)) "一\n- 二\n"))
    (peta-journal--goto-item "b" 0)
    (should (equal (peta-journal--id-at (point)) "b"))))

;;;; Protecting the marks

;; バレットと項目の区切りは編集できない。項目をまたぐ削除は、エラーになる。
(ert-deftest peta-journal-the-marks-between-items-cannot-be-edited ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (should-error (delete-region (point-min) (point-max)) :type 'text-read-only)
    (goto-char (point-min))
    (should-error (insert "z") :type 'text-read-only)
    (should (equal (buffer-string) "• x\n• y\n"))))

;; 空の項目にも、本文の先頭と末尾に入力できる。
(ert-deftest peta-journal-an-empty-item-takes-text ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 ""))
    (peta-journal-test--goto "a" 0)
    (insert "x")
    (should (equal (buffer-string) "• x\n"))))

;;;; The point

;; 上の項目の行末から下の行へ移ると、バレットを飛ばして本文の先頭に入る。
(ert-deftest peta-journal-the-point-skips-a-bullet-going-forward ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal-test--goto "a" 1)
    (forward-char 1)
    (peta-journal--post-command)
    (should (equal (peta-journal--id-at (point)) "b"))
    (should (= (peta-journal--cursor) 0))))

;; 本文の先頭から戻ると、上の項目の末尾へ移る。
(ert-deftest peta-journal-the-point-crosses-a-bullet-going-backward ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy")
                                      (peta-journal-test--row "b" 1 "z"))
    (peta-journal-test--goto "b" 0)
    (backward-char 1)
    (peta-journal--post-command)
    (should (equal (peta-journal--id-at (point)) "a"))
    (should (= (peta-journal--cursor) 2))))

;; 先頭の項目の先頭から戻っても、そこに留まる。
(ert-deftest peta-journal-the-point-stays-at-the-start-of-the-first-item ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 0)
    (backward-char 1)
    (peta-journal--post-command)
    (should (= (peta-journal--cursor) 0))))

;; バッファの末尾へ移っても、最後の項目の末尾に留まる。
(ert-deftest peta-journal-the-point-stays-in-the-last-item-at-the-end ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 0)
    (goto-char (point-max))
    (peta-journal--post-command)
    (should (equal (peta-journal--id-at (point)) "a"))
    (should (= (peta-journal--cursor) 1))))

;; 日付の行には留まらず、進む向きの項目へ移る。
(ert-deftest peta-journal-the-point-passes-over-a-date-header ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x")
            (peta-journal-test--row "b" 1 "y" :header "2026-11-05"))
    (peta-journal-test--goto "a" 1)
    (forward-char 1)
    (peta-journal--post-command)
    (should (equal (peta-journal--id-at (point)) "b"))
    (backward-char 1)
    (peta-journal--post-command)
    (should (equal (peta-journal--id-at (point)) "a"))))

;;;; What goes to the core

;; 別の項目へ移ると、前の編集を終え、移った先の編集を始める。
(ert-deftest peta-journal-moving-between-items-switches-the-edit ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yz"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-test--goto "b" 1)
    (should (equal peta-journal-test--sent
                   '(("edit_finished" :id "a")
                     ("edit_requested" :id "b" :cursor 1))))))

;; 同じ項目の中を動いても、何も送らない。
(ert-deftest peta-journal-moving-inside-an-item-sends-nothing ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xyz"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-test--goto "a" 2)
    (should (null peta-journal-test--sent))))

;; 入力すると、その項目の本文全体がcoreへ送られる。
(ert-deftest peta-journal-typing-sends-the-whole-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (insert "日")
    (should (equal peta-journal-test--sent
                   '(("body_changed" :id "a" :body "x日y"))))))

;; 編集を始める前に入力されても、先に編集を始めてから本文を送る。
(ert-deftest peta-journal-typing-starts-the-edit-first ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (goto-char (peta-journal--body-start (point-min)))
    (insert "z")
    (should (equal (mapcar #'car peta-journal-test--sent)
                   '("edit_requested" "body_changed")))))

;; 打った文字にも、続きの行の字下げが付く。
(ert-deftest peta-journal-typed-lines-get-the-indent-of-their-item ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 2 "x"))
    (peta-journal-test--goto "a" 1)
    (insert "\ny")
    (should (equal (get-text-property (line-beginning-position) 'line-prefix) "    "))))

;; 届いた行が自分の入力より古いとき、入力中の本文を上書きしない。
(ert-deftest peta-journal-an-old-answer-does-not-overwrite-typed-text ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal-test--goto "a" 1)
    (insert "z")
    (setq peta-journal--seq 5)
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yy"))
                              "page" 4)
    (should (equal (buffer-string) "• xz\n• yy\n"))))

;; 同じ番号まで処理された答えは、入力中の項目にも反映する。
(ert-deftest peta-journal-a-current-answer-replaces-the-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal--seq 5)
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "new")) "page" 5)
    (should (equal (buffer-string) "• new\n"))))

;; RETは、カーソルの位置を添えてcoreへ送る。
(ert-deftest peta-journal-ret-sends-enter-with-the-cursor ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-enter)
    (should (equal peta-journal-test--sent '(("enter" :id "a" :cursor 1))))
    (should (equal (buffer-string) "• xy\n"))))

;; 本文の先頭のDELは、上の項目との結合としてcoreへ送る。途中では文字を消す。
(ert-deftest peta-journal-del-joins-items-only-at-the-start-of-a-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-backspace)
    (should (equal (buffer-string) "• y\n"))
    (setq peta-journal-test--sent nil)
    (peta-journal-backspace)
    (should (equal peta-journal-test--sent '(("backspace_at_start" :id "a"))))))

;; 本文の末尾のC-dは、下の項目との結合としてcoreへ送る。
(ert-deftest peta-journal-delete-joins-items-only-at-the-end-of-a-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (peta-journal-delete)
    (should (equal (buffer-string) "• x\n"))
    (setq peta-journal-test--sent nil)
    (peta-journal-delete)
    (should (equal peta-journal-test--sent '(("delete_at_end" :id "a"))))))

;; C-yは、キルリングの先頭をcoreの貼り付けとして送る。文字の属性は渡さない。
(ert-deftest peta-journal-yank-pastes-through-the-core ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    ;; Without this, the kill comes from the clipboard of the OS
    (let ((interprogram-paste-function nil)
          (kill-ring (list (propertize "貼" 'face 'bold))))
      (peta-journal-yank))
    (should (equal peta-journal-test--sent '(("paste" :id "a" :text "貼" :cursor 1))))
    (should-not (text-properties-at 0 (plist-get (cdar peta-journal-test--sent) :text)))))

;; 手で編集を終えると保存が走り、次の移動で編集がまた始まる。
(ert-deftest peta-journal-saving-ends-the-edit-until-the-next-command ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 0)
    (setq peta-journal-test--sent nil)
    (peta-journal-save)
    (should (equal peta-journal-test--sent '(("edit_finished" :id "a"))))
    (setq peta-journal-test--sent nil)
    ;; 保存のコマンド自身の直後には、編集を開き直さない。
    (let ((this-command 'peta-journal-save))
      (peta-journal--post-command))
    (should (null peta-journal-test--sent))
    (peta-journal--post-command)
    (should (equal peta-journal-test--sent '(("edit_requested" :id "a" :cursor 0))))))

;; 別のページを開くコマンドの直後は、離れるページの項目で編集を始めない。
(ert-deftest peta-journal-going-to-another-page-leaves-the-edit-closed ()
  (dolist (command '(peta-journal-today peta-journal-find-page peta-journal-back
                     peta-journal-next-day peta-journal-previous-day
                     peta-journal-open-at-point))
    (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
      (peta-journal-test--goto "a" 0)
      (peta-journal--go "back")
      (setq peta-journal-test--sent nil)
      (let ((this-command command))
        (peta-journal--post-command))
      (should (null peta-journal-test--sent)))))

;; 一息つくと編集を閉じる。開いたままだと、coreは他からの変更の読み直しを待ち続けるため。
(ert-deftest peta-journal-idling-ends-the-edit ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 0)
    (setq peta-journal-test--sent nil)
    (peta-journal--finish-edit)
    (should (equal peta-journal-test--sent '(("edit_finished" :id "a"))))
    (setq peta-journal-test--sent nil)
    (peta-journal--finish-edit)
    (should (null peta-journal-test--sent))))

;;;; Answers of the shell

;; 新しいカーソル位置の指示で、その項目の指定の位置へ移る。編集中の項目もそれに合わせる。
(ert-deftest peta-journal-a-focus-moves-the-point-and-the-edit ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yz"))
    (peta-journal-test--goto "a" 0)
    (setq peta-journal-test--sent nil)
    (peta-journal--on-focus "b" 1 0)
    (should (equal (peta-journal--id-at (point)) "b"))
    (should (= (peta-journal--cursor) 1))
    (peta-journal--post-command)
    (should (null peta-journal-test--sent))))

;; 位置のない指示は、すでにその項目にあるカーソルを動かさない。別の項目なら末尾へ移す。
(ert-deftest peta-journal-a-focus-without-a-cursor-keeps-the-point-in-its-item ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xyz")
                                      (peta-journal-test--row "b" 1 "uv"))
    (peta-journal-test--goto "a" 1)
    (peta-journal--on-focus "a" nil 0)
    (should (= (peta-journal--cursor) 1))
    (peta-journal--on-focus "b" nil 0)
    (should (equal (peta-journal--id-at (point)) "b"))
    (should (= (peta-journal--cursor) 2))))

;; その後の操作をまだ処理していない指示は、カーソルを引き戻さない。
(ert-deftest peta-journal-an-old-focus-does-not-pull-the-point-back ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (peta-journal-test--goto "b" 0)
    (setq peta-journal--seq 5)
    (peta-journal--on-focus "a" 0 4)
    (should (equal (peta-journal--id-at (point)) "b"))))

;; 届いた行は、途中で切れていても、改行まで揃ってから読まれる。
(ert-deftest peta-journal-lines-from-the-shell-may-arrive-in-pieces ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal--read-output "{\"t\":\"title\",\"ti")
    (should (equal peta-journal--title ""))
    (peta-journal--read-output "tle\":\"題\"}\n")
    (should (equal peta-journal--title "題"))))

;;;; Starting

;; 別のファイルの読み込み中に起動しても、プログラムはこのモードの場所から見つかる。
(ert-deftest peta-journal-the-program-is-found-while-another-file-loads ()
  (let* ((peta-journal-shell-program nil)
         (load-file-name "c:/elsewhere/init.el")
         (here (file-name-directory (locate-library "peta-journal")))
         (program (expand-file-name "../../../target/debug/peta-shell-emacs" here)))
    (cl-letf (((symbol-function 'executable-find) #'ignore)
              ((symbol-function 'file-executable-p) (lambda (file) (equal file program))))
      (should (equal (peta-journal--program) program)))))

;;;; The search box

(defmacro peta-journal-test--with-search (&rest body)
  "Run BODY with an open list of search results."
  (declare (indent 0))
  `(let ((peta-journal--searching nil))
     (peta-journal--search-open)
     (unwind-protect
         (progn ,@body)
       (peta-journal--search-close))))

(defun peta-journal-test--entries (&rest titles)
  "Search results of the kind page, with TITLES."
  (vconcat (seq-map-indexed
            (lambda (title index)
              (list :index index :kind "page" :title title :snippet ""))
            titles)))

(defun peta-journal-test--search-text ()
  "What the list of search results shows, without its marks."
  (with-current-buffer "*PetaJournal Search*"
    (substring-no-properties (buffer-string))))

;; 検索窓は C-c / で呼び出す。
(ert-deftest peta-journal-the-search-box-is-on-a-short-key ()
  (with-temp-buffer
    (peta-journal-mode)
    (should (eq (key-binding (kbd "C-c /")) #'peta-journal-search))))

;; 結果は入力に合わせて窓に並ぶ。ページ、新規作成、項目はそれぞれの形で示される。
(ert-deftest peta-journal-results-show-pages-creates-and-items ()
  (peta-journal-test--with-search
    (peta-journal--on-search
     (vector (list :index 0 :kind "page" :title "設計メモ" :snippet "")
             (list :index 1 :kind "create" :title "設計" :snippet "")
             (list :index 2 :kind "item" :title "Quarkus" :snippet "<b>設計</b>の方針"))
     nil)
    (should (equal (peta-journal-test--search-text)
                   "  設計メモ\n  [新規] 設計\n  Quarkus: 設計の方針\n"))))

;; 先頭の結果が選ばれていて、C-n と C-p で動く。端では反対の端へ回る。
(ert-deftest peta-journal-the-selection-moves-and-wraps ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a" "b" "c") nil)
    (should (= (peta-journal--search-choice) 0))
    (peta-journal-search-next)
    (should (= (peta-journal--search-choice) 1))
    (peta-journal-search-previous)
    (peta-journal-search-previous)
    (should (= (peta-journal--search-choice) 2))
    (peta-journal-search-next)
    (should (= (peta-journal--search-choice) 0))))

;; 選ぶ番号は、行の順番ではなくコアが付けた番号である。
(ert-deftest peta-journal-the-choice-is-the-number-from-the-core ()
  (peta-journal-test--with-search
    (peta-journal--on-search
     (vector (list :index 7 :kind "page" :title "a" :snippet "")) nil)
    (should (= (peta-journal--search-choice) 7))))

;; 同じ結果がまた届いても選択は動かない。結果が変われば先頭へ戻る。
(ert-deftest peta-journal-the-selection-stays-until-the-results-change ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a" "b") t)
    (peta-journal-search-next)
    (peta-journal--on-search (peta-journal-test--entries "a" "b") nil)
    (should (= (peta-journal--search-choice) 1))
    (peta-journal--on-search (peta-journal-test--entries "b" "c") nil)
    (should (= (peta-journal--search-choice) 0))))

;; 検索語が空になると結果は消え、選べるものがなくなる。
(ert-deftest peta-journal-an-empty-query-empties-the-list ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a") nil)
    (peta-journal--on-search [] nil)
    (should (equal (peta-journal-test--search-text) ""))
    (should (null (peta-journal--search-choice)))))

;; 窓を閉じたあとに届いた結果では、何も開かれない。
(ert-deftest peta-journal-results-after-closing-the-box-are-ignored ()
  (peta-journal--search-close)
  (peta-journal--on-search (peta-journal-test--entries "a") nil)
  (should-not (get-buffer "*PetaJournal Search*")))

;; 検索中かどうかは、シェルの言葉どおりに覚える。
(ert-deftest peta-journal-remembers-whether-a-search-is-running ()
  (let ((peta-journal--searching nil))
    (peta-journal--on-search [] t)
    (should peta-journal--searching)
    (peta-journal--on-search [] nil)
    (should-not peta-journal--searching)))

;;;; The shell stopping

(defun peta-journal-test--emacs-process (sentinel &rest args)
  "Start a batch Emacs with ARGS and SENTINEL, as a stand-in for the shell."
  (make-process :name "peta-test-process"
                :command (append (list (expand-file-name invocation-name invocation-directory)
                                       "--batch")
                                 args)
                :connection-type 'pipe
                :noquery t
                :sentinel sentinel))

;; シェルが思いがけず止まると、以後の入力は保存されない。それをモード行と警告で伝える。
(ert-deftest peta-journal-warns-when-the-shell-stops ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (let ((process (peta-journal-test--emacs-process #'ignore))
          (warning nil))
      (while (process-live-p process)
        (accept-process-output process 0.02))
      (cl-letf (((symbol-function 'display-warning)
                 (lambda (_type message &rest _) (setq warning message))))
        (peta-journal--sentinel process "exited abnormally with code 1\n"))
      (should (string-match-p "exited abnormally with code 1" warning))
      (should (equal peta-journal--connection "shell stopped")))))

;; こちらから止めたときは、警告を出さない。
(ert-deftest peta-journal-stays-silent-when-told-to-stop-the-shell ()
  (let* ((warning nil)
         ;; Ends when its input closes, as the shell does
         (peta-journal--process (peta-journal-test--emacs-process
                                 #'peta-journal--sentinel "--eval" "(read-string \"\")")))
    (cl-letf (((symbol-function 'display-warning)
               (lambda (_type message &rest _) (setq warning message))))
      (peta-journal-stop)
      (accept-process-output nil 0.05))
    (should-not (process-live-p peta-journal--process))
    (should-not warning)))

;;;; Closing what was typed

;; `[`と`]`とバッククォートを打つと、カーソルの位置をシェルへ伝える。他の文字では伝えない。
(ert-deftest peta-journal-reports-only-the-characters-that-may-need-closing ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 ""))
    (peta-journal-test--goto "a" 0)
    (peta-journal-test--type "x[]`")
    (should (equal (peta-journal-test--sent-of "typed")
                   '(("typed" :id "a" :cursor 2)
                     ("typed" :id "a" :cursor 3)
                     ("typed" :id "a" :cursor 4))))))

;;;; The color of what was typed

(defun peta-journal-test--body-has-no-color-p (id)
  "Non-nil when no character in the body of the item ID has a color of its own."
  (let ((start (peta-journal--find-row id)))
    (not (text-property-not-all (peta-journal--body-start start)
                                (peta-journal--body-end start)
                                'font-lock-face nil))))

;; 空の項目に打った文字は、バレットの色を受け継がない。
(ert-deftest peta-journal-text-typed-in-an-empty-item-does-not-take-the-bullet-color ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 ""))
    (peta-journal-test--goto "a" 0)
    (peta-journal-test--type "ab")
    (should (equal (buffer-string) "• ab\n"))
    (should (peta-journal-test--body-has-no-color-p "a"))))

;; 完了タスクの本文の先頭に打っても、色を受け継がない。バレットの色はそのままである。
(ert-deftest peta-journal-text-typed-before-a-body-does-not-take-the-bullet-color ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy" :bullet "done"))
    (peta-journal-test--goto "a" 0)
    (peta-journal-test--type "ab")
    (should (equal (buffer-string) "☑ abxy\n"))
    (should (peta-journal-test--body-has-no-color-p "a"))
    (should (eq (get-text-property (peta-journal--find-row "a") 'font-lock-face)
                'peta-journal-done))))

;;;; The color of Flat Markdown

(defun peta-journal-test--face (body text)
  "The face of the start of TEXT, in a row that has BODY."
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 body))
    (font-lock-ensure)
    (goto-char (point-min))
    (search-forward text)
    (get-text-property (match-beginning 0) 'face)))

;; 強調の記法には、それぞれの色が付く。
(ert-deftest peta-journal-emphasis-has-its-color ()
  (should (eq (peta-journal-test--face "a **太字** b" "太字") 'peta-journal-bold))
  (should (eq (peta-journal-test--face "a *斜体* b" "斜体") 'peta-journal-italic))
  (should (eq (peta-journal-test--face "a _斜体_ b" "斜体") 'peta-journal-italic))
  (should (eq (peta-journal-test--face "a __下線__ b" "下線") 'peta-journal-underline))
  (should (eq (peta-journal-test--face "a ~~取消~~ b" "取消") 'peta-journal-strike))
  (should (eq (peta-journal-test--face "a ~下付~ b" "下付") 'peta-journal-script))
  (should (eq (peta-journal-test--face "a ^上付^ b" "上付") 'peta-journal-script))
  (should (eq (peta-journal-test--face "a ==目立つ== b" "目立つ") 'peta-journal-highlight))
  (should (eq (peta-journal-test--face "a ||伏せ字|| b" "伏せ字") 'peta-journal-spoiler)))

;; 強調にならない記号の並びには、色が付かない。
(ert-deftest peta-journal-delimiters-that-do-not-emphasize-have-no-color ()
  (should-not (peta-journal-test--face "a * x * b" "x"))
  (should-not (peta-journal-test--face "a_b_c" "b"))
  (should-not (peta-journal-test--face "a ===x=== b" "x"))
  (should-not (peta-journal-test--face "a ~~~x~~~ b" "x"))
  (should-not (peta-journal-test--face "a \\*x* b" "x")))

;;;; Links

;; リンクを開くコマンドは、編集を閉じてから、カーソルの位置をシェルへ送る。リンクかどうかはcoreが決める。
(ert-deftest peta-journal-sends-the-cursor-to-open-a-link ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[x]]"))
    (peta-journal-test--goto "a" 6)
    (setq peta-journal--current "a"
          peta-journal-test--sent nil)
    (peta-journal-open-at-point)
    (should (equal peta-journal-test--sent
                   '(("edit_finished" :id "a")
                     ("open_link_at" :id "a" :cursor 6))))))

;;; peta-journal-test.el ends here
