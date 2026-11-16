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
     (rename-buffer "*PetaJournal*" t)
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

(defun peta-journal-test--shown ()
  "The text of the buffer, with the space behind each bullet as a plain space."
  (string-replace " " " " (buffer-string)))

(defun peta-journal-test--sent-of (type)
  "The lines of TYPE sent so far."
  (cl-remove-if-not (lambda (line) (equal (car line) type)) peta-journal-test--sent))

;;;; Drawing

;; 各項目は、バレットと本文の1行として描かれる。バッファの文字に字下げは入らない。
(ert-deftest peta-journal-a-row-is-a-bullet-and-its-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "親")
                                      (peta-journal-test--row "b" 2 "子"))
    (should (equal (peta-journal-test--shown) "• 親\n• 子\n"))))

;; 字下げは表示だけで、レベルの分の幅になる。
(ert-deftest peta-journal-the-indent-of-a-row-follows-its-level ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "親")
                                      (peta-journal-test--row "b" 3 "孫"))
    (peta-journal--goto-item "b" 0)
    (should (equal (get-text-property (line-beginning-position) 'line-prefix) "    "))))

;; タスクにもバレットがあり、状態の記号はその後ろに付く。
(ert-deftest peta-journal-a-task-has-its-mark-behind-the-bullet ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x" :task "task")
            (peta-journal-test--row "b" 1 "y" :task "running")
            (peta-journal-test--row "c" 1 "z" :task "done"))
    (should (equal (peta-journal-test--shown) "• ☐ x\n• ⊟ y\n• ☑ z\n"))))

;; 折りたたんだ項目は、丸で囲んだ点のバレットで示される。行末には何も足さない。
(ert-deftest peta-journal-the-bullet-shows-a-collapsed-item ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x" :collapsed t)
            (peta-journal-test--row "b" 1 "y"))
    (should (equal (peta-journal-test--shown) "◉ x\n• y\n"))
    (should-not (get-text-property (peta-journal--body-end (peta-journal--find-row "a"))
                                   'display))))

;; トランスクルージョンは、四角で囲んだ点のバレットで示される。折りたたむと、四角が塗られる。
(ert-deftest peta-journal-the-bullet-shows-a-transclusion ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x" :bullet "transclusion")
            (peta-journal-test--row "b" 1 "y" :bullet "transclusion" :collapsed t))
    (should (equal (peta-journal-test--shown) "⊡ x\n▣ y\n"))))

;; タスクのバレットも、折りたたみとトランスクルージョンを示す。
(ert-deftest peta-journal-the-bullet-of-a-task-shows-what-its-row-is ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "x" :task "task" :collapsed t)
            (peta-journal-test--row "b" 1 "y" :task "done" :bullet "transclusion")
            (peta-journal-test--row "c" 1 "z" :task "running" :bullet "transclusion"
                                    :collapsed t)
            (peta-journal-test--row "d" 1 "w" :task "task" :bullet "broken"))
    (should (equal (peta-journal-test--shown) "◉ ☐ x\n⊡ ☑ y\n▣ ⊟ z\n✗ ☐ w\n"))))

;; ジャーナルの日付は、項目の上の編集できない行である。
(ert-deftest peta-journal-a-date-header-is-a-line-above-its-row ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "今日")
            (peta-journal-test--row "b" 1 "昨日" :header "2026-11-05"))
    (should (equal (peta-journal-test--shown) "• 今日\n── 2026-11-05 ──\n• 昨日\n"))
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
      (should (equal (peta-journal-test--shown) "• x\n• yy\n"))
      (should (= (marker-position marker) (peta-journal--body-start (point-min)))))))

;; 変わった行より後ろの行も書き換えられない。行が増えても減っても同じである。
(ert-deftest peta-journal-the-rows-after-a-change-are-not-rewritten ()
  (dolist (new (list (list (peta-journal-test--row "a" 1 "x")
                           (peta-journal-test--row "b" 1 "yy")
                           (peta-journal-test--row "c" 1 "z"))
                     (list (peta-journal-test--row "a" 1 "x")
                           (peta-journal-test--row "n" 1 "新")
                           (peta-journal-test--row "b" 1 "y")
                           (peta-journal-test--row "c" 1 "z"))
                     (list (peta-journal-test--row "a" 1 "x")
                           (peta-journal-test--row "c" 1 "z"))))
    (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                        (peta-journal-test--row "b" 1 "y")
                                        (peta-journal-test--row "c" 1 "z"))
      (let ((marker (copy-marker (peta-journal--body-start (peta-journal--find-row "c")))))
        (peta-journal--apply-rows (vconcat new) "page" 0)
        (should (= (marker-position marker)
                   (peta-journal--body-start (peta-journal--find-row "c"))))))))

;; 同じ行だけが届けば、どの行も書き換えられない。
(ert-deftest peta-journal-the-same-rows-rewrite-nothing ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "y"))
    (let ((first-row (copy-marker (peta-journal--body-start (peta-journal--find-row "a"))))
          (second-row (copy-marker (peta-journal--body-start (peta-journal--find-row "b")))))
      (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "x")
                                        (peta-journal-test--row "b" 1 "y"))
                                "page" 0)
      (should (= first-row (peta-journal--body-start (peta-journal--find-row "a"))))
      (should (= second-row (peta-journal--body-start (peta-journal--find-row "b")))))))

;; 同じ内容の行が並んでいても、先頭と末尾の一致を二重に数えず、増減が正しく映る。
(ert-deftest peta-journal-rows-with-the-same-content-are-counted-once ()
  (let ((row (peta-journal-test--row "a" 1 "x")))
    (peta-journal-test--with-rows (list row row)
      (peta-journal--apply-rows (vector row row row) "page" 0)
      (should (equal (peta-journal-test--shown) "• x\n• x\n• x\n"))
      (peta-journal--apply-rows (vector row) "page" 0)
      (should (equal (peta-journal-test--shown) "• x\n")))))

;; 空のページに行が届いても、すべての行が消えても、見える行は届いた行と同じである。
(ert-deftest peta-journal-rows-can-go-from-none-to-some-and-back ()
  (peta-journal-test--with-rows nil
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "x")) "page" 0)
    (should (equal (peta-journal-test--shown) "• x\n"))
    (peta-journal--apply-rows (vector) "page" 0)
    (should (equal (peta-journal-test--shown) ""))))

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
    (should (equal (peta-journal-test--shown) "• x\n• y\n"))))

;; 空の項目にも、本文の先頭と末尾に入力できる。
(ert-deftest peta-journal-an-empty-item-takes-text ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 ""))
    (peta-journal-test--goto "a" 0)
    (insert "x")
    (should (equal (peta-journal-test--shown) "• x\n"))))

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

;; 編集中の項目が届いた行から消えると、その編集は終わっている。終わりは送らない。
(ert-deftest peta-journal-a-row-that-is-gone-is-no-longer-edited ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yz"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal--apply-rows (vector (peta-journal-test--row "b" 1 "yz")) "page" 0)
    (peta-journal--post-command)
    (should (equal peta-journal-test--sent
                   '(("edit_requested" :id "b" :cursor 0))))))

;; 項目の削除は、coreに頼むだけである。断られることがあるので、編集中の項目はそのままにする。
(ert-deftest peta-journal-deleting-an-item-only-asks-the-core ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yz"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-delete-item)
    (peta-journal--post-command)
    (should (equal peta-journal-test--sent '(("delete_item" :id "a"))))))

;; Stashへ送るのも、coreに頼むだけである。行は、サーバーの答えのあとで動く。
(ert-deftest peta-journal-sending-to-the-stash-only-asks-the-core ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x")
                                      (peta-journal-test--row "b" 1 "yz"))
    (peta-journal-test--goto "b" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-send-to-stash)
    (peta-journal--post-command)
    (should (equal peta-journal-test--sent '(("send_to_stash" :id "b"))))))

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
    (should (equal (peta-journal-test--shown) "• xz\n• yy\n"))))

;; 同じ番号まで処理された答えは、入力中の項目にも反映する。
(ert-deftest peta-journal-a-current-answer-replaces-the-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal--seq 5)
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "new")) "page" 5)
    (should (equal (peta-journal-test--shown) "• new\n"))))

;; RETは、カーソルの位置を添えてcoreへ送る。
(ert-deftest peta-journal-ret-sends-enter-with-the-cursor ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-enter)
    (should (equal peta-journal-test--sent '(("enter" :id "a" :cursor 1))))
    (should (equal (peta-journal-test--shown) "• xy\n"))))

;; 本文の先頭のDELは、上の項目との結合としてcoreへ送る。途中では文字を消す。
(ert-deftest peta-journal-del-joins-items-only-at-the-start-of-a-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-backspace)
    (should (equal (peta-journal-test--shown) "• y\n"))
    (setq peta-journal-test--sent nil)
    (peta-journal-backspace)
    (should (equal peta-journal-test--sent '(("backspace_at_start" :id "a"))))))

;; 本文の末尾のC-dは、下の項目との結合としてcoreへ送る。
(ert-deftest peta-journal-delete-joins-items-only-at-the-end-of-a-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (peta-journal-delete)
    (should (equal (peta-journal-test--shown) "• x\n"))
    (setq peta-journal-test--sent nil)
    (peta-journal-delete)
    (should (equal peta-journal-test--sent '(("delete_at_end" :id "a"))))))

(defmacro peta-journal-test--with-clipboard (text image &rest body)
  "Run BODY with TEXT and the PNG bytes IMAGE in the clipboard of the OS.
Either can be nil, for a clipboard without it."
  (declare (indent 2))
  `(let ((text ,text)
         (image ,image))
     (cl-letf (((symbol-function 'gui-get-selection)
                (lambda (_selection type)
                  (pcase type
                    ('TARGETS (vconcat (and text '(UTF8_STRING STRING))
                                       (and image '(BITMAP image/png))))
                    ((or 'UTF8_STRING 'STRING) text)
                    ('image/png image)))))
       ,@body)))

(defun peta-journal-test--bytes-of (file)
  "The bytes in FILE, as a unibyte string."
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert-file-contents-literally file)
    (buffer-string)))

;; C-yは、キルリングの先頭をcoreの貼り付けとして送る。文字の属性は渡さない。
(ert-deftest peta-journal-yank-pastes-through-the-core ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    ;; Without these, the kill comes from the clipboard of the OS
    (peta-journal-test--with-clipboard nil nil
      (let ((interprogram-paste-function nil)
            (kill-ring (list (propertize "貼" 'face 'bold))))
        (peta-journal-yank)))
    (should (equal peta-journal-test--sent '(("paste" :id "a" :text "貼" :cursor 1))))
    (should-not (text-properties-at 0 (plist-get (cdar peta-journal-test--sent) :text)))))

;; クリップボードに画像だけがあるとき、C-yはその画像を挿入する。
;; 画像は、中身のまま`image.png`という名前のファイルになり、項目とカーソルと一緒にシェルへ送られる。
;; ファイルは、挿入が済むと消える。文字の貼り付けは送らない。
(ert-deftest peta-journal-yank-inserts-the-image-in-the-clipboard ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    ;; Bytes that break when written as text: a line end, a zero and a high byte
    (let ((bytes (unibyte-string #x89 ?P ?N ?G 13 10 0 255))
          (collect (symbol-function 'peta-journal--send))
          (uploaded nil))
      (cl-letf (((symbol-function 'peta-journal--send)
                 (lambda (type &rest fields)
                   (apply collect type fields)
                   ;; The shell reads the file while Emacs waits for it
                   (setq uploaded (peta-journal-test--bytes-of (plist-get fields :path))))))
        (peta-journal-test--with-clipboard nil bytes
          (let ((interprogram-paste-function nil)
                (kill-ring (list "古い文字")))
            (peta-journal-yank))))
      (pcase-let ((`((,type . ,fields)) peta-journal-test--sent))
        (should (equal type "insert_image"))
        (should (equal (plist-get fields :id) "a"))
        (should (equal (plist-get fields :cursor) 1))
        (should (equal (file-name-nondirectory (plist-get fields :path)) "image.png"))
        (should (file-name-absolute-p (plist-get fields :path)))
        (should-not (file-exists-p (plist-get fields :path))))
      (should (equal uploaded bytes)))))

;; クリップボードに文字もあるときは、画像があっても、文字を貼り付ける。
(ert-deftest peta-journal-yank-pastes-the-text-when-the-clipboard-has-an-image-as-well ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (peta-journal-test--with-clipboard "表" (unibyte-string #x89 ?P ?N ?G)
      (let ((interprogram-paste-function nil)
            (kill-ring (list "表")))
        (peta-journal-yank)))
    (should (equal peta-journal-test--sent '(("paste" :id "a" :text "表" :cursor 1))))))

;; 選んだ画像は、項目とカーソルと絶対パスとしてシェルへ送る。拡張子の大文字小文字は問わない。
(ert-deftest peta-journal-inserting-an-image-sends-the-item-the-cursor-and-the-path ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (cl-letf (((symbol-function 'read-file-name) (lambda (&rest _) "pic.PNG")))
      (call-interactively #'peta-journal-insert-image))
    (should (equal peta-journal-test--sent
                   `(("insert_image" :id "a"
                      :path ,(expand-file-name "pic.PNG")
                      :cursor 1))))))

;; 画像でないファイルは、サーバーへ送る前に断る。
(ert-deftest peta-journal-a-file-that-is-no-image-is-refused ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (setq peta-journal-test--sent nil)
    (cl-letf (((symbol-function 'read-file-name) (lambda (&rest _) "notes.txt")))
      (should-error (call-interactively #'peta-journal-insert-image) :type 'user-error))
    (should (null peta-journal-test--sent))))

;; ファイルの候補には画像とフォルダだけが出る。フォルダは、中へ進むために要る。
(ert-deftest peta-journal-the-file-choice-offers-images-and-folders ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy"))
    (peta-journal-test--goto "a" 1)
    (let (offered)
      (cl-letf (((symbol-function 'read-file-name)
                 (lambda (_prompt _dir _default _mustmatch _initial predicate)
                   (setq offered predicate)
                   "pic.png")))
        (call-interactively #'peta-journal-insert-image))
      (should (funcall offered "写真.jpeg"))
      (should (funcall offered temporary-file-directory))
      (should-not (funcall offered "notes.txt"))
      (should-not (funcall offered "png")))))

;; 項目の外では、ファイルを尋ねる前に断る。選んだファイルが無駄にならない。
(ert-deftest peta-journal-inserting-an-image-outside-an-item-refuses-before-asking ()
  (peta-journal-test--with-rows nil
    (setq peta-journal-test--sent nil)
    (cl-letf (((symbol-function 'read-file-name)
               (lambda (&rest _) (error "It must not ask"))))
      (should-error (call-interactively #'peta-journal-insert-image) :type 'user-error))
    (should (null peta-journal-test--sent))))

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
  (dolist (command '(peta-journal-today peta-journal-back peta-journal-forward
                     peta-journal-next-day peta-journal-previous-day
                     peta-journal-open-at-point peta-journal-rename-page
                     peta-journal-delete-page))
    (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
      (peta-journal-test--goto "a" 0)
      (peta-journal--go "back")
      (setq peta-journal-test--sent nil)
      (let ((this-command command))
        (peta-journal--post-command))
      (should (null peta-journal-test--sent)))))

;; 名前の変更は、今のパスを初期値にして尋ね、編集を閉じてから新しいパスを送る。
(ert-deftest peta-journal-renaming-offers-the-path-and-ends-the-edit ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal--on-line '(:t "page_path" :path "work/memo"))
    (peta-journal-test--goto "a" 0)
    (setq peta-journal-test--sent nil)
    (let ((offered nil))
      (cl-letf (((symbol-function 'read-string)
                 (lambda (_prompt initial &rest _)
                   (setq offered initial)
                   "work/new")))
        (call-interactively #'peta-journal-rename-page))
      (should (equal offered "work/memo")))
    (should (equal peta-journal-test--sent
                   '(("edit_finished" :id "a")
                     ("rename_page" :path "work/new"))))))

;; 今のページの削除は、編集を閉じてからシェルへ送る。尋ねるのはコアである。
(ert-deftest peta-journal-deleting-the-page-ends-the-edit-and-asks-the-shell ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal-test--goto "a" 0)
    (setq peta-journal-test--sent nil)
    (with-temp-buffer
      (call-interactively #'peta-journal-delete-page))
    (should (equal peta-journal-test--sent
                   '(("edit_finished" :id "a")
                     ("delete_page"))))))

;; 変えられるパスがなければ、入力を求める前に断る。打った名前が無駄にならない。
(ert-deftest peta-journal-a-page-without-a-path-refuses-before-asking ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal--on-line '(:t "page_path" :path nil))
    (setq peta-journal-test--sent nil)
    (cl-letf (((symbol-function 'read-string)
               (lambda (&rest _) (error "It must not ask"))))
      (should-error (call-interactively #'peta-journal-rename-page) :type 'user-error))
    (should (null peta-journal-test--sent))))

;; ページのバッファの外で M-x で呼んでも、そのページの名前を変える。
(ert-deftest peta-journal-renaming-from-another-buffer-is-about-the-page ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal--on-line '(:t "page_path" :path "memo"))
    (setq peta-journal-test--sent nil)
    (cl-letf (((symbol-function 'read-string) (lambda (_prompt initial &rest _) initial)))
      (with-temp-buffer
        (call-interactively #'peta-journal-rename-page)))
    (should (equal peta-journal-test--sent '(("rename_page" :path "memo"))))))

;; シェルを起動し直すと、前のシェルが伝えたパスは忘れる。次のページが名前を変えられるとは限らない。
(ert-deftest peta-journal-a-new-shell-forgets-the-path ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (peta-journal--on-line '(:t "page_path" :path "memo"))
    (let ((stand-in (peta-journal-test--emacs-process #'ignore)))
      (unwind-protect
          (cl-letf (((symbol-function 'peta-journal--program) #'ignore)
                    ((symbol-function 'make-process) (lambda (&rest _) stand-in)))
            (peta-journal--start-shell "/today"))
        (delete-process stand-in)))
    (should (null peta-journal--page-path))))

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

;;;; The gap between items

;; 項目の末尾の改行にだけ、設定の分の隙間を付ける。文字は増えない。
(ert-deftest peta-journal-the-newline-at-the-end-of-an-item-has-the-gap ()
  (let ((peta-journal-item-gap 7))
    (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
      (should (= (get-text-property (peta-journal--body-end (peta-journal--find-row "a"))
                                    'line-spacing)
                 7))
      (should (equal (peta-journal-test--shown) "• x\n")))))

;; 本文の中の改行には、隙間を付けない。1つの項目の行は詰まったままである。
(ert-deftest peta-journal-a-newline-inside-a-body-has-no-gap ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x\ny"))
    (goto-char (point-min))
    (search-forward "\n")
    (should-not (get-text-property (1- (point)) 'line-spacing))))

;;;; Wrapping

;; 折り返しの行に、フリンジのマークを出さない。他のマークと、他のバッファの設定はそのままである。
(ert-deftest peta-journal-wrapped-lines-have-no-marks-in-the-fringes ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (should (equal (cdr (assq 'continuation fringe-indicator-alist)) nil))
    (should (assq 'truncation fringe-indicator-alist)))
  (with-temp-buffer
    (should (cdr (assq 'continuation fringe-indicator-alist)))))

(defun peta-journal-test--on-first-screen-line-p (position)
  "Non-nil when POSITION is on the first screen line of the buffer.
Not `vertical-motion': in a batch run it does not know about `word-wrap'."
  (= (cdr (window-text-pixel-size nil (point-min) 2))
     (cdr (window-text-pixel-size nil (point-min) (1+ position)))))

(defmacro peta-journal-test--with-long-row (body &rest forms)
  "Run FORMS with a row of BODY in the selected window."
  (declare (indent 1))
  `(peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 ,body))
     (set-window-buffer (selected-window) (current-buffer))
     ,@forms))

;; 空白のない長い日本語の本文は、バレットの行から始まる。バレットだけの行を作らない。
(ert-deftest peta-journal-long-japanese-starts-on-the-line-of-the-bullet ()
  (peta-journal-test--with-long-row (make-string 300 ?あ)
    (should (peta-journal-test--on-first-screen-line-p
             (peta-journal--body-start (peta-journal--find-row "a"))))))

;; 本文の途中に空白があっても、そこで行を空けずに日本語を続ける。
(ert-deftest peta-journal-long-japanese-follows-a-space-on-the-same-line ()
  (peta-journal-test--with-long-row (concat "abc " (make-string 300 ?あ))
    (should (peta-journal-test--on-first-screen-line-p
             (+ (peta-journal--body-start (peta-journal--find-row "a")) 4)))))

;; 長いURLのような本文も、バレットの行から始まる。
(ert-deftest peta-journal-a-long-word-starts-on-the-line-of-the-bullet ()
  (peta-journal-test--with-long-row (concat "https://example.com/" (make-string 300 ?a))
    (should (peta-journal-test--on-first-screen-line-p
             (peta-journal--body-start (peta-journal--find-row "a"))))))

;; バレットの後ろの改行しない空白に、Emacsの強調を付けない。普通の空白に見せるためである。
(ert-deftest peta-journal-the-space-behind-the-bullet-is-not-highlighted ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (should-not nobreak-char-display)))

;;;; Page properties

(defconst peta-journal-test--property-rows
  [(:key "created" :label "created" :value "2026-11-07" :removable nil :editable t)
   (:key "modified" :label "modified" :value "2026-11-08" :removable nil :editable nil)
   (:key "status" :label "status" :value "draft" :removable t :editable t)]
  "The rows of a doc page, as the shell sends them.")

(defmacro peta-journal-test--with-properties (rows free-keys &rest body)
  "Run BODY in a page buffer that was told ROWS and FREE-KEYS."
  (declare (indent 2))
  `(peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
     (unwind-protect
         (progn
           (peta-journal--on-line (list :t "properties" :rows ,rows :free_keys ,free-keys))
           (setq peta-journal-test--sent nil)
           ,@body)
       (when (get-buffer "*PetaJournal Properties*")
         (kill-buffer "*PetaJournal Properties*")))))

(defun peta-journal-test--properties-text ()
  "What the list of properties shows."
  (with-current-buffer "*PetaJournal Properties*"
    (substring-no-properties (buffer-string))))

;; `C-c C-x p`で、プロパティの一覧を別の窓に開く。カーソルは一覧の最初のプロパティに移るので、
;; そのまま`s`や`d`を押せる。
(ert-deftest peta-journal-the-list-shows-the-properties-and-takes-the-point ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (let ((page (current-buffer)))
      (set-window-buffer (selected-window) page)
      (execute-kbd-macro (kbd "C-c C-x p"))
      (should (equal (buffer-name (window-buffer (selected-window))) "*PetaJournal Properties*"))
      (should (equal (buffer-substring-no-properties (line-beginning-position) (line-end-position))
                     "created   2026-11-07"))
      (should (equal (peta-journal-test--properties-text)
                     (concat "Properties\n\n"
                             "created   2026-11-07\n"
                             "modified  2026-11-08\n"
                             "status    draft\n"))))))

;; もう一度`C-c C-x p`を押すと、窓を閉じる。
(ert-deftest peta-journal-the-key-closes-the-open-list ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (set-window-buffer (selected-window) (current-buffer))
    (peta-journal-properties)
    (peta-journal-properties)
    (should-not (get-buffer-window "*PetaJournal Properties*"))))

;; 一覧を開いたまま値が変わると、一覧も変わる。別のページのバッファからの知らせでは変わらない。
(ert-deftest peta-journal-the-open-list-follows-its-page ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (set-window-buffer (selected-window) (current-buffer))
    ;; The list takes the point, but the shell speaks to the page
    (save-current-buffer (peta-journal-properties))
    (peta-journal--on-line
     '(:t "properties"
       :rows [(:key "status" :label "status" :value "done" :removable t :editable t)]
       :free_keys []))
    (should (string-search "status  done" (peta-journal-test--properties-text)))
    (peta-journal-test--with-rows (list (peta-journal-test--row "b" 1 "y"))
      (peta-journal--on-line
       '(:t "properties"
         :rows [(:key "other" :label "other" :value "z" :removable t :editable t)]
         :free_keys [])))
    (should-not (string-search "other" (peta-journal-test--properties-text)))))

;; ジャーナルには、プロパティの一覧がない。
(ert-deftest peta-journal-a-journal-has-no-list-of-properties ()
  (peta-journal-test--with-properties nil []
    (should-error (peta-journal-properties) :type 'user-error)))

;; シェルを起動し直すと、前のシェルが伝えたプロパティは忘れる。
(ert-deftest peta-journal-a-new-shell-forgets-the-properties ()
  (peta-journal-test--with-properties peta-journal-test--property-rows [(:key "rating" :label "rating")]
    (let ((stand-in (peta-journal-test--emacs-process #'ignore)))
      (unwind-protect
          (cl-letf (((symbol-function 'peta-journal--program) #'ignore)
                    ((symbol-function 'make-process) (lambda (&rest _) stand-in)))
            (peta-journal--start-shell "/today"))
        (delete-process stand-in)))
    (should-error (peta-journal-properties) :type 'user-error)))

;; 一覧は、キーを、名前の本体と型の見出しで見せる。
(ert-deftest peta-journal-the-list-shows-the-label-of-each-key ()
  (peta-journal-test--with-properties
      [(:key "score_n" :label "score (number)" :value "5" :removable t :editable t)]
      []
    (peta-journal-properties)
    (should (string-search "score (number)  5\n" (peta-journal-test--properties-text)))))

;; 全角のキーがあっても、値の列がそろう。
(ert-deftest peta-journal-the-list-lines-up-wide-keys ()
  (peta-journal-test--with-properties
      [(:key "状態" :label "状態" :value "a" :removable t :editable t)
       (:key "status" :label "status" :value "b" :removable t :editable t)]
      []
    (peta-journal-properties)
    (should (string-search "状態    a\nstatus  b\n" (peta-journal-test--properties-text)))))

;; ジャーナルへ移ると、開いていた一覧からページの値が消える。
(ert-deftest peta-journal-the-open-list-empties-on-a-journal ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (save-current-buffer (peta-journal-properties))
    (peta-journal--on-line '(:t "properties" :rows nil :free_keys []))
    (should-not (string-search "status" (peta-journal-test--properties-text)))))

(defmacro peta-journal-test--answering (key value &rest body)
  "Run BODY while the minibuffer answers KEY and VALUE.
`offered' holds the keys it offered, and `initial' the value it started with."
  (declare (indent 2))
  `(let ((offered nil) (initial nil))
     (cl-letf (((symbol-function 'completing-read)
                (lambda (_prompt collection &rest _)
                  (setq offered collection)
                  ,key))
               ((symbol-function 'read-string)
                (lambda (_prompt start &rest _)
                  (setq initial start)
                  ,value)))
       ,@body)))

;; 設定では、変えられるキーと、まだ付いていないキーを補完する。今の値を初期値にして、一覧も開く。
(ert-deftest peta-journal-setting-offers-keys-and-the-current-value ()
  (peta-journal-test--with-properties peta-journal-test--property-rows [(:key "rating" :label "rating")]
    (peta-journal-test--answering "status" "done"
      (call-interactively #'peta-journal-set-property)
      (should (equal (all-completions "" offered) '("created" "status" "rating")))
      (should (equal initial "draft")))
    (should (equal peta-journal-test--sent '(("set_property" :key "status" :value "done"))))
    (should (get-buffer-window "*PetaJournal Properties*"))))

(defmacro peta-journal-test--answering-the-default (value &rest body)
  "Run BODY while the minibuffer takes the key it offers by default, and answers VALUE."
  (declare (indent 1))
  `(cl-letf (((symbol-function 'completing-read)
              (lambda (_prompt _collection &optional _predicate _require _initial _history default
                               &rest _)
                default))
             ((symbol-function 'read-string) (lambda (&rest _) ,value)))
     ,@body))

;; 一覧の中で`s`を押すと、カーソルの行のキーを既定にして、値を設定する。
(ert-deftest peta-journal-s-in-the-list-sets-the-property-on-the-line ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (set-window-buffer (selected-window) (current-buffer))
    (save-current-buffer
      (peta-journal-properties)
      (forward-line 2)
      (peta-journal-test--answering-the-default "done"
        (execute-kbd-macro (kbd "s"))))
    (should (equal peta-journal-test--sent '(("set_property" :key "status" :value "done"))))))

;; 設定したあとも、一覧が書き直されたあとも、カーソルは同じ行に残る。続けて同じキーを直せる。
(ert-deftest peta-journal-the-list-keeps-the-line-after-a-change ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (let ((page (current-buffer)))
      (set-window-buffer (selected-window) page)
      (save-current-buffer
        (peta-journal-properties)
        (forward-line 2)
        (peta-journal-test--answering-the-default "done"
          (execute-kbd-macro (kbd "s"))))
      (peta-journal--on-line
       '(:t "properties"
         :rows [(:key "created" :label "created" :value "2026-11-07" :removable nil :editable t)
                (:key "modified" :label "modified" :value "2026-11-08" :removable nil :editable nil)
                (:key "status" :label "status" :value "done" :removable t :editable t)]
         :free_keys []))
      (with-selected-window (get-buffer-window "*PetaJournal Properties*")
        (should (equal (buffer-substring-no-properties (line-beginning-position)
                                                       (line-end-position))
                       "status    done"))))))

;; 一覧の中で`d`を押すと、カーソルの行のキーを既定にして、そのプロパティを消す。
(ert-deftest peta-journal-d-in-the-list-removes-the-property-on-the-line ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (set-window-buffer (selected-window) (current-buffer))
    (save-current-buffer
      (peta-journal-properties)
      (forward-line 2)
      (peta-journal-test--answering-the-default ""
        (execute-kbd-macro (kbd "d"))))
    (should (equal peta-journal-test--sent '(("delete_property" :key "status"))))))

;; 設定では、キーを見出しで補完し、選んだ見出しのキーを送る。
(ert-deftest peta-journal-setting-offers-the-labels-and-sends-the-key ()
  (peta-journal-test--with-properties
      [(:key "rating_n" :label "rating (number)" :value "3" :removable t :editable t)]
      [(:key "rating_d" :label "rating (date)")]
    (peta-journal-test--answering "rating (date)" "2026-11-15"
      (call-interactively #'peta-journal-set-property)
      (should (equal (all-completions "" offered) '("rating (number)" "rating (date)"))))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "rating_d" :value "2026-11-15"))))))

;; まだ付いていないキーの初期値は空である。
(ert-deftest peta-journal-a-new-key-starts-empty ()
  (peta-journal-test--with-properties peta-journal-test--property-rows [(:key "rating" :label "rating")]
    (peta-journal-test--answering "rating" "5"
      (call-interactively #'peta-journal-set-property)
      (should (null initial)))
    (should (equal peta-journal-test--sent '(("set_property" :key "rating" :value "5"))))))

(defmacro peta-journal-test--with-dates-in-japan (&rest body)
  "Run BODY in Japan, on a page with dates in `created' and `met', and `published' to add."
  (declare (indent 0))
  `(let ((zone (getenv "TZ")))
     (unwind-protect
         (peta-journal-test--with-properties peta-journal-test--property-rows []
           (set-time-zone-rule "JST-9")
           (peta-journal--on-line
            '(:t "properties"
              :rows [(:key "created" :label "created" :value "2026-11-07T15:21:04.123Z"
                      :removable nil :editable t)
                     (:key "modified" :label "modified" :value "2026-11-08T00:00:00Z"
                      :removable nil :editable nil)
                     (:key "met" :label "met" :value "soon" :removable t :editable t)
                     (:key "status" :label "status" :value "2026-11-07T15:21:04Z"
                      :removable t :editable t)]
              :free_keys [(:key "published" :label "published")]
              :datetime_keys ["created" "modified" "met" "published"]))
           ,@body)
       (set-time-zone-rule zone))))

;; 日時の値は、一覧ではローカル時刻で分まで見せる。日時のキーでも読めない値と、日時でないキーの値は、そのまま見せる。
(ert-deftest peta-journal-the-list-shows-dates-in-local-time ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-properties)
    (should (equal (peta-journal-test--properties-text)
                   (concat "Properties\n\n"
                           "created   2026-11-08 00:21\n"
                           "modified  2026-11-08 09:00\n"
                           "met       soon\n"
                           "status    2026-11-07T15:21:04Z\n")))))

;; 日時の値はローカル時刻を初期値にして尋ね、打った時刻はUTCにして送る。
(ert-deftest peta-journal-a-date-is-asked-in-local-time-and-sent-in-utc ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-test--answering "created" "2026-11-09 08:05"
      (call-interactively #'peta-journal-set-property)
      (should (equal initial "2026-11-08 00:21")))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "created" :value "2026-11-08T23:05:00Z"))))))

;; 初期値のままEnterを押すと、今の値をそのまま送る。分までの表示のせいで秒を失わない。
(ert-deftest peta-journal-a-date-left-as-it-is-keeps-its-seconds ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-test--answering "created" "2026-11-08 00:21"
      (call-interactively #'peta-journal-set-property))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "created" :value "2026-11-07T15:21:04.123Z"))))))

;; まだ付いていない日時のキーは現在時刻を初期値にして尋ねるので、Enterだけで今の時刻が入る。
(ert-deftest peta-journal-a-new-date-starts-at-now ()
  (peta-journal-test--with-dates-in-japan
    (cl-letf (((symbol-function 'current-time)
               (lambda () (date-to-time "2026-11-08T00:30:45Z"))))
      (peta-journal-test--answering "published" initial
        (call-interactively #'peta-journal-set-property)
        (should (equal initial "2026-11-08 09:30"))))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "published" :value "2026-11-08T00:30:00Z"))))))

;; 日時として読めない入力は、送らずに断る。ありえない日付も繰り上げずに断る。
(ert-deftest peta-journal-a-date-that-cannot-be-read-is-refused ()
  (peta-journal-test--with-dates-in-japan
    (dolist (typed '("tomorrow" "2026-11-08" "2026-13-01 00:00" "2026-02-30 00:00"
                     "2026-11-08 24:00" "2026-11-08T09:30"))
      (peta-journal-test--answering "created" typed
        (should-error (call-interactively #'peta-journal-set-property) :type 'user-error)))
    (should (null peta-journal-test--sent))))

;; 日時のキーでも、値が空なら、他のキーと同じように断る。
(ert-deftest peta-journal-an-empty-date-is-refused-as-any-empty-value ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-test--answering "created" ""
      (should-error (call-interactively #'peta-journal-set-property) :type 'user-error))
    (should (null peta-journal-test--sent))))

;; 読めない値が入っている日時のキーは、初期値のままなら今の値を送り、直せば日時として送る。
(ert-deftest peta-journal-a-date-key-with-another-value-can-be-corrected ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-test--answering "met" "2026-11-08 09:00"
      (call-interactively #'peta-journal-set-property)
      (should (equal initial "soon")))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "met" :value "2026-11-08T00:00:00Z"))))))

;; 日時でないキーは、日時の形の値でも、打ったまま送る。
(ert-deftest peta-journal-a-key-that-holds-no-date-is-sent-as-typed ()
  (peta-journal-test--with-dates-in-japan
    (peta-journal-test--answering "status" "2026-11-08 09:00"
      (call-interactively #'peta-journal-set-property)
      (should (equal initial "2026-11-07T15:21:04Z")))
    (should (equal peta-journal-test--sent
                   '(("set_property" :key "status" :value "2026-11-08 09:00"))))))

;; キーを選ばなかったときと、値が空のときは、何も送らない。値を消すのは削除の役目である。
(ert-deftest peta-journal-setting-refuses-an-empty-key-or-value ()
  (peta-journal-test--with-properties peta-journal-test--property-rows [(:key "rating" :label "rating")]
    (peta-journal-test--answering "" "5"
      (should-error (call-interactively #'peta-journal-set-property) :type 'user-error))
    (peta-journal-test--answering "rating" ""
      (should-error (call-interactively #'peta-journal-set-property) :type 'user-error))
    (should (null peta-journal-test--sent))))

;; ジャーナルでは、キーを尋ねずに断る。
(ert-deftest peta-journal-a-journal-refuses-a-property-before-asking ()
  (peta-journal-test--with-properties nil [(:key "rating" :label "rating")]
    (cl-letf (((symbol-function 'completing-read) (lambda (&rest _) (error "It must not ask"))))
      (should-error (call-interactively #'peta-journal-set-property) :type 'user-error)
      (should-error (call-interactively #'peta-journal-delete-property) :type 'user-error))))

;; 削除では、消せるキーだけを補完する。キーを選ばなければ、何も送らない。
(ert-deftest peta-journal-deleting-offers-the-removable-keys ()
  (peta-journal-test--with-properties peta-journal-test--property-rows []
    (peta-journal-test--answering "" ""
      (should-error (call-interactively #'peta-journal-delete-property) :type 'user-error))
    (should (null peta-journal-test--sent))
    (peta-journal-test--answering "status" ""
      (call-interactively #'peta-journal-delete-property)
      (should (equal (all-completions "" offered) '("status"))))
    (should (equal peta-journal-test--sent '(("delete_property" :key "status"))))
    (should (get-buffer-window "*PetaJournal Properties*"))))

;; 消せるプロパティがなければ、尋ねずに断る。
(ert-deftest peta-journal-a-page-without-removable-keys-refuses-the-delete ()
  (peta-journal-test--with-properties
      [(:key "created" :label "created" :value "2026-11-07" :removable nil :editable t)]
      []
    (cl-letf (((symbol-function 'completing-read) (lambda (&rest _) (error "It must not ask"))))
      (should-error (call-interactively #'peta-journal-delete-property) :type 'user-error))))

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
         (program (expand-file-name "../../../../target/debug/outliner-emacs" here)))
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
              (list :position index :kind "page" :title title :label title
                    :snippet ""))
            titles)))

(defun peta-journal-test--choice ()
  "The number the core gave to the selected result, or nil."
  (plist-get (peta-journal--search-entry) :position))

(defun peta-journal-test--search-text ()
  "What the list of search results shows, without its marks."
  (with-current-buffer "*PetaJournal Search*"
    (substring-no-properties (buffer-string))))

;; 結果は入力に合わせて窓に並ぶ。各行は、コアが決めた文言である。項目には、その後に抜粋が付く。
(ert-deftest peta-journal-results-show-the-labels-of-the-core ()
  (peta-journal-test--with-search
    (peta-journal--on-search
     (vector (list :position 0 :kind "page" :title "設計メモ" :label "設計メモ"
                   :snippet "")
             (list :position 1 :kind "create" :title "設計"
                   :label "Create page \"設計\"" :snippet "")
             (list :position 2 :kind "item" :title "Quarkus" :label "Quarkus"
                   :snippet "<b>設計</b>の方針"))
     nil)
    (should (equal (peta-journal-test--search-text)
                   "  設計メモ\n  Create page \"設計\"\n  Quarkus: 設計の方針\n"))))

;; 先頭の結果が選ばれていて、C-n と C-p で動く。端では反対の端へ回る。
(ert-deftest peta-journal-the-selection-moves-and-wraps ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a" "b" "c") nil)
    (should (= (peta-journal-test--choice) 0))
    (peta-journal-search-next)
    (should (= (peta-journal-test--choice) 1))
    (peta-journal-search-previous)
    (peta-journal-search-previous)
    (should (= (peta-journal-test--choice) 2))
    (peta-journal-search-next)
    (should (= (peta-journal-test--choice) 0))))

;; 選ぶ番号は、行の順番ではなくコアが付けた番号である。
(ert-deftest peta-journal-the-choice-is-the-number-from-the-core ()
  (peta-journal-test--with-search
    (peta-journal--on-search
     (vector (list :position 7 :kind "page" :title "a" :label "a" :snippet "")) nil)
    (should (= (peta-journal-test--choice) 7))))

;; 同じ結果がまた届いても選択は動かない。結果が変われば先頭へ戻る。
(ert-deftest peta-journal-the-selection-stays-until-the-results-change ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a" "b") t)
    (peta-journal-search-next)
    (peta-journal--on-search (peta-journal-test--entries "a" "b") nil)
    (should (= (peta-journal-test--choice) 1))
    (peta-journal--on-search (peta-journal-test--entries "b" "c") nil)
    (should (= (peta-journal-test--choice) 0))))

;; 検索語が空になると結果は消え、選べるものがなくなる。
(ert-deftest peta-journal-an-empty-query-empties-the-list ()
  (peta-journal-test--with-search
    (peta-journal--on-search (peta-journal-test--entries "a") nil)
    (peta-journal--on-search [] nil)
    (should (equal (peta-journal-test--search-text) ""))
    (should (null (peta-journal-test--choice)))))

;; 入力が空のときに`TAB`を押すと、最近のページが、その順のまま結果の一覧に並ぶ。先頭が選ばれている。
(ert-deftest peta-journal-tab-on-an-empty-query-lists-the-recent-pages ()
  (should (eq (keymap-lookup peta-journal-search-minibuffer-map "TAB")
              #'peta-journal-search-recent))
  (peta-journal-test--with-search
    (let ((peta-journal--recent '("zeta" "alpha")))
      (with-temp-buffer
        (peta-journal-search-recent))
      (should (equal (peta-journal-test--search-text) "  zeta\n  alpha\n"))
      (with-current-buffer "*PetaJournal Search*"
        (should (= peta-journal--search-selected 0))))))

;; 検索語を入力してあるときの`TAB`は、検索の結果をそのまま残す。
(ert-deftest peta-journal-tab-after-a-query-keeps-the-results ()
  (peta-journal-test--with-search
    (let ((peta-journal--recent '("zeta")))
      (peta-journal--on-search (peta-journal-test--entries "a") nil)
      (with-temp-buffer
        (insert "設計")
        (peta-journal-search-recent))
      (should (equal (peta-journal-test--search-text) "  a\n")))))

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
      (process-put process 'peta-page (current-buffer))
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

;;;; Two buffers

;; シェルからの行は、そのシェルのバッファだけを変える。
(ert-deftest peta-journal-a-line-from-a-shell-changes-only-its-own-buffer ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "最初"))
    (let ((first (current-buffer)))
      (peta-journal-test--with-rows (list (peta-journal-test--row "b" 1 "二番目"))
        (peta-journal--on-line (list :t "rows"
                                     :rows (vector (peta-journal-test--row "b" 1 "変更"))
                                     :page_id "page"
                                     :ack 0))
        (should (equal (peta-journal-test--shown) "• 変更\n"))
        (should (equal (with-current-buffer first (peta-journal-test--shown)) "• 最初\n"))))))

;; 一息つくと、どのバッファの編集も閉じる。
(ert-deftest peta-journal-an-idle-moment-ends-the-edit-in-every-buffer ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (setq peta-journal--current "a")
    (peta-journal-test--with-rows (list (peta-journal-test--row "b" 1 "y"))
      (setq peta-journal--current "b")
      (peta-journal--idle)
      (let ((sent (peta-journal-test--sent-of "edit_finished")))
        (should (= (length sent) 2))
        (should (member '("edit_finished" :id "a") sent))
        (should (member '("edit_finished" :id "b") sent))))))

;; シェルが止まったことは、そのシェルのバッファに表示される。
(ert-deftest peta-journal-a-stopped-shell-is-shown-in-its-own-buffer ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (let ((first (current-buffer)))
      (peta-journal-test--with-rows (list (peta-journal-test--row "b" 1 "y"))
        (let ((process (peta-journal-test--emacs-process #'ignore)))
          (process-put process 'peta-page (current-buffer))
          (while (process-live-p process)
            (accept-process-output process 0.02))
          (cl-letf (((symbol-function 'display-warning) #'ignore))
            (peta-journal--sentinel process "finished\n"))
          (should (equal peta-journal--connection "shell stopped"))
          (should (equal (buffer-local-value 'peta-journal--connection first) "")))))))

;; 他のバッファが表示中のページへ移ると、そのことを一度だけ伝える。保存しても、互いの画面には届かないためである。
(ert-deftest peta-journal-warns-when-another-buffer-shows-the-page ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (let ((first (buffer-name))
          (rows (vector (peta-journal-test--row "b" 1 "y")))
          (messages nil))
      (peta-journal-test--with-rows (list (peta-journal-test--row "b" 1 "y"))
        (peta-journal--apply-rows rows "other" 0)
        (cl-letf (((symbol-function 'message)
                   (lambda (format-string &rest args)
                     (push (apply #'format format-string args) messages))))
          (peta-journal--apply-rows rows "another" 0)
          (should-not messages)
          (peta-journal--apply-rows rows "page" 0)
          (should (= (length messages) 1))
          (should (string-search first (car messages)))
          (peta-journal--apply-rows rows "page" 0)
          (should (= (length messages) 1)))))))

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
    (should (equal (peta-journal-test--shown) "• ab\n"))
    (should (peta-journal-test--body-has-no-color-p "a"))))

;; 完了タスクの本文の先頭に打っても、色を受け継がない。タスクの記号の色はそのままである。
(ert-deftest peta-journal-text-typed-before-a-body-does-not-take-the-bullet-color ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "xy" :task "done"))
    (peta-journal-test--goto "a" 0)
    (peta-journal-test--type "ab")
    (should (equal (peta-journal-test--shown) "• ☑ abxy\n"))
    (should (peta-journal-test--body-has-no-color-p "a"))
    (should (equal peta-journal-test--sent
                   '(("edit_requested" :id "a" :cursor 0)
                     ("body_changed" :id "a" :body "axy")
                     ("body_changed" :id "a" :body "abxy"))))
    (should (eq (get-text-property (+ (peta-journal--find-row "a") 2) 'font-lock-face)
                'peta-journal-done))))

;; 実行中のタスクの記号は、大きく表示するfaceを持つ。ほかの状態の四角より小さく見えるためである。
(ert-deftest peta-journal-the-mark-of-a-running-task-has-a-face-of-its-own ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x" :task "running")
                                      (peta-journal-test--row "b" 1 "y" :task "task"))
    (should (eq (get-text-property (+ (peta-journal--find-row "a") 2) 'font-lock-face)
                'peta-journal-running))
    (should (eq (get-text-property (+ (peta-journal--find-row "b") 2) 'font-lock-face)
                'peta-journal-bullet))
    (should (> (face-attribute 'peta-journal-running :height) 1))))

;; カーソルは、タスクの記号の上にも留まらない。
(ert-deftest peta-journal-the-point-does-not-rest-on-the-mark-of-a-task ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x" :task "task"))
    (goto-char (+ (peta-journal--find-row "a") 2))
    (peta-journal--post-command)
    (should (= (peta-journal--cursor) 0))
    (should (equal (peta-journal--body-at (point)) "x"))))

;; タスクの本文の先頭の記法にも、色が付く。
(ert-deftest peta-journal-the-start-of-the-body-of-a-task-has-its-color ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "> 引用" :task "task")
            (peta-journal-test--row "b" 1 "> 引用" :task "done" :bullet "transclusion"
                                    :collapsed t))
    (font-lock-ensure)
    (goto-char (point-min))
    (dotimes (_ 2)
      (search-forward "引用")
      (should (eq (get-text-property (match-beginning 0) 'face) 'peta-journal-quote)))))

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

;; インラインコードと数式には色が付き、その中の記号は修飾にならない。
(ert-deftest peta-journal-inline-code-has-its-color-and-holds-no-decoration ()
  (should (eq (peta-journal-test--face "a `コード` b" "コード") 'peta-journal-code))
  (should (eq (peta-journal-test--face "a `` x`y `` b" "x`y") 'peta-journal-code))
  (should (eq (peta-journal-test--face "a $`x^2`$ b" "x^2") 'peta-journal-math))
  (should (eq (peta-journal-test--face "a `**x**` b" "x") 'peta-journal-code))
  (should (eq (peta-journal-test--face "a `#tag [[y]]` b" "#tag") 'peta-journal-code))
  (should (eq (peta-journal-test--face "a `#tag [[y]]` b" "[[y") 'peta-journal-code)))

;; コードブロックは、閉じる行までの全体に色が付き、中の記号は修飾にならない。
(ert-deftest peta-journal-a-code-block-has-its-color-up-to-its-closing ()
  (let ((body "```rust\n**x**\n```\nc"))
    (should (eq (peta-journal-test--face body "```rust") 'peta-journal-code))
    (should (eq (peta-journal-test--face body "**x") 'peta-journal-code))
    (should-not (peta-journal-test--face body "c")))
  (should (eq (peta-journal-test--face "~~~\n*x*\n~~~" "*x") 'peta-journal-code))
  (should (eq (peta-journal-test--face "````\n```\nx\n````" "x") 'peta-journal-code))
  (should (eq (peta-journal-test--face "a\n```\nx\n```" "x") 'peta-journal-code)))

;; 閉じていないコードブロックは本文の末尾までで、次の項目へ広がらない。
(ert-deftest peta-journal-an-open-code-block-ends-with-its-item ()
  (should (eq (peta-journal-test--face "```\nx" "x") 'peta-journal-code))
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "```\nx")
                                      (peta-journal-test--row "b" 1 "y"))
    (font-lock-ensure)
    (goto-char (point-min))
    (search-forward "y")
    (should-not (get-text-property (match-beginning 0) 'face))))

;; 本文の先頭の記法は、どのバレットの後ろでも色が付く。
(ert-deftest peta-journal-the-start-of-a-body-has-its-color-behind-any-bullet ()
  (peta-journal-test--with-rows
      (list (peta-journal-test--row "a" 1 "> 引用" :collapsed t)
            (peta-journal-test--row "b" 1 "> 引用" :bullet "transclusion")
            (peta-journal-test--row "c" 1 "> 引用" :bullet "transclusion" :collapsed t))
    (font-lock-ensure)
    (goto-char (point-min))
    (dotimes (_ 3)
      (search-forward "引用")
      (should (eq (get-text-property (match-beginning 0) 'face) 'peta-journal-quote)))))

;; 字下げのコードにも色が付く。
(ert-deftest peta-journal-indented-code-has-its-color ()
  (should (eq (peta-journal-test--face "    x = 1\n    y = 2" "y") 'peta-journal-code)))

;; 引用とアラートには色が付く。引用の中の修飾は、修飾の色になる。
(ert-deftest peta-journal-quotes-and-alerts-have-their-colors ()
  (should (eq (peta-journal-test--face "> 引用" "引用") 'peta-journal-quote))
  (should (eq (peta-journal-test--face "> a\n> 次の行" "次の行") 'peta-journal-quote))
  (should (eq (peta-journal-test--face "> **太字**" "太字") 'peta-journal-bold))
  (let ((body ">>>\n複数行\n>>>\n後"))
    (should (eq (peta-journal-test--face body "複数行") 'peta-journal-quote))
    (should-not (peta-journal-test--face body "後")))
  (should (eq (peta-journal-test--face "> [!WARNING] 注意" "注意") 'peta-journal-alert))
  (should (eq (peta-journal-test--face "> [!note]" "note") 'peta-journal-alert))
  (let ((body ">>> [!TIP]\n本文\n>>>"))
    (should (eq (peta-journal-test--face body "[!TIP") 'peta-journal-alert))
    (should (eq (peta-journal-test--face body "本文") 'peta-journal-quote))))

;; リストとタスクの記号、水平線、小さい文字には色が付く。リストの本文には付かない。
(ert-deftest peta-journal-list-marks-rules-and-subtext-have-their-colors ()
  (should (eq (peta-journal-test--face "- 項目" "-") 'peta-journal-list-mark))
  (should-not (peta-journal-test--face "- 項目" "項目"))
  (should (eq (peta-journal-test--face "* 項目" "*") 'peta-journal-list-mark))
  (should (eq (peta-journal-test--face "12. 項目" "12.") 'peta-journal-list-mark))
  (should (eq (peta-journal-test--face "- a\n  - 子" "- 子") 'peta-journal-list-mark))
  (should (eq (peta-journal-test--face "- [ ] やる" "[ ]") 'peta-journal-list-mark))
  (should (eq (peta-journal-test--face "- [x] 済み" "[x]") 'peta-journal-list-mark))
  (should (eq (peta-journal-test--face "---" "---") 'peta-journal-rule))
  (should (eq (peta-journal-test--face "***" "***") 'peta-journal-rule))
  (should (eq (peta-journal-test--face "-# 小さい" "小さい") 'peta-journal-subtext)))

;; 表は、区切りの行があるときだけ色が付き、表の続きの行までである。セルの中の修飾は、修飾の色になる。
(ert-deftest peta-journal-a-table-has-its-color ()
  (let ((body "| 見 | b |\n|:--|:-:|\n| 中 | d |\n後"))
    (should (eq (peta-journal-test--face body "見") 'peta-journal-table))
    (should (eq (peta-journal-test--face body "|:--") 'peta-journal-table))
    (should (eq (peta-journal-test--face body "中") 'peta-journal-table))
    (should-not (peta-journal-test--face body "後")))
  (should (eq (peta-journal-test--face "見 | b\n--|--" "見") 'peta-journal-table))
  (should-not (peta-journal-test--face "| 見 | b |" "見"))
  (should (eq (peta-journal-test--face "| **太** |\n|--|" "太") 'peta-journal-bold)))

;; 見出しは6段までで、本文のどの行でもよい。下線の見出しもある。中の修飾は、修飾の色になる。
(ert-deftest peta-journal-a-heading-has-its-color ()
  (should (eq (peta-journal-test--face "# 見" "見") 'peta-journal-heading))
  (should (eq (peta-journal-test--face "###### 見" "見") 'peta-journal-heading))
  (should-not (peta-journal-test--face "####### 見" "見"))
  (should (eq (peta-journal-test--face "a\n# 見" "見") 'peta-journal-heading))
  (should (eq (peta-journal-test--face "題\n===" "題") 'peta-journal-heading))
  (should (eq (peta-journal-test--face "題\n---" "---") 'peta-journal-heading))
  (should (eq (peta-journal-test--face "# **太** [[y]]" "太") 'peta-journal-bold))
  (should (eq (peta-journal-test--face "# **太** [[y]]" "[[y") 'peta-journal-link)))

;; タグは、文頭か空白の直後から、タグに使える文字までである。
(ert-deftest peta-journal-a-tag-has-its-color-up-to-where-it-ends ()
  (should (eq (peta-journal-test--face "#タグ" "タグ") 'peta-journal-link))
  (should (eq (peta-journal-test--face "a #親/子-1 b" "子-1") 'peta-journal-link))
  (should-not (peta-journal-test--face "a #タグ。本文" "本文"))
  (should-not (peta-journal-test--face "a#b" "b"))
  (should-not (peta-journal-test--face "\\#b" "b")))

;; リンク、画像、脚注、絵文字、HTMLには色が付く。URLの中の記号は修飾にならない。
(ert-deftest peta-journal-links-and-embedded-things-have-their-colors ()
  (should (eq (peta-journal-test--face "[題](https://x.jp)" "題") 'peta-journal-link))
  (should (eq (peta-journal-test--face "a https://x.jp/b_c_d e" "c_d") 'peta-journal-link))
  (should (eq (peta-journal-test--face "a www.x.jp b" "www") 'peta-journal-link))
  (should (eq (peta-journal-test--face "a me@x.jp b" "me@") 'peta-journal-link))
  (should (eq (peta-journal-test--face "![写真](a_b_c.png#w=300)" "写真") 'peta-journal-image))
  (should (eq (peta-journal-test--face "![図\\[1\\]](a.png)" "png") 'peta-journal-image))
  (should (eq (peta-journal-test--face "[版 [2]](https://x.jp)" "x.jp") 'peta-journal-link))
  (should (eq (peta-journal-test--face "注[^1]" "[^1") 'peta-journal-footnote))
  (should (eq (peta-journal-test--face "[^1]: 注記" "[^1") 'peta-journal-footnote))
  (should (eq (peta-journal-test--face "a ^[脚注] b" "脚注") 'peta-journal-footnote))
  (should (eq (peta-journal-test--face "a :heart_eyes: b" "heart") 'peta-journal-emoji))
  (should (eq (peta-journal-test--face "a :+1: b" "+1") 'peta-journal-emoji))
  (should-not (peta-journal-test--face "12:30:45" "30"))
  (should (eq (peta-journal-test--face "<b>太</b>" "<b") 'peta-journal-html))
  (should-not (peta-journal-test--face "<b>太</b>" "太"))
  (should (eq (peta-journal-test--face "<br />" "<br") 'peta-journal-html)))

;; 行の一部だけを色付けし直しても、複数行の修飾は本文全体から判断される。
(ert-deftest peta-journal-a-partial-fontify-sees-the-whole-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "```\nx\n```")
                                      (peta-journal-test--row "b" 1 "y"))
    (font-lock-set-defaults)
    (goto-char (point-min))
    (search-forward "x")
    (font-lock-fontify-region (match-beginning 0) (match-end 0))
    (should (eq (get-text-property (match-beginning 0) 'face) 'peta-journal-code))
    (goto-char (point-min))
    (search-forward "```")
    (should (eq (get-text-property (match-beginning 0) 'face) 'peta-journal-code))))

;; コードブロックを打ち始めると、打った行より後の行の色も変わる。
(ert-deftest peta-journal-opening-a-block-recolors-the-lines-after-it ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x\ny"))
    (font-lock-set-defaults)
    (peta-journal-test--goto "a" 0)
    (peta-journal-test--type "```\n")
    (font-lock-fontify-region (point) (1+ (point)))
    (goto-char (point-min))
    (search-forward "y")
    (should (eq (get-text-property (match-beginning 0) 'face) 'peta-journal-code))))

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

;;;; Images

(defconst peta-journal-test--png (unibyte-string #x89 ?P ?N ?G 13 10 26 10)
  "The start of a PNG file: enough for Emacs to tell the type in a batch run.")

(defvar peta-journal-test--fetched nil
  "The URLs a test asked for, oldest first.")

(defmacro peta-journal-test--with-images (rows &rest body)
  "Run BODY on ROWS as `peta-journal-test--with-rows', on a display with images.
Nothing is fetched: a test hands over the answer with `peta-journal--image-arrived'."
  (declare (indent 1))
  `(cl-letf (((symbol-function 'display-images-p) (lambda (&rest _) t))
             ((symbol-function 'peta-journal--fetch-image)
              (lambda (url)
                (setq peta-journal-test--fetched
                      (append peta-journal-test--fetched (list url))))))
     (clrhash peta-journal--images)
     (setq peta-journal-test--fetched nil)
     (peta-journal-test--with-rows ,rows ,@body)))

(defun peta-journal-test--asset-url (name)
  "The URL of the asset NAME."
  (concat peta-journal-server "/assets/" name))

(defun peta-journal-test--image-of (id)
  "The image shown in the item ID, or nil."
  (let ((start (peta-journal--find-row id)))
    (seq-some (lambda (overlay) (overlay-get overlay 'display))
              (overlays-in start (peta-journal--body-end start)))))

;; カーソルのない項目では、リンクの代わりに画像が見える。バッファの文字は変わらない。
(ert-deftest peta-journal-an-item-without-the-point-shows-its-image ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (should (equal (image-property (peta-journal-test--image-of "b") :data)
                   peta-journal-test--png))
    (should (equal (peta-journal-test--shown) "• 上\n• ![p](../assets/p.png)\n"))))

;; 画像は、リンクの部分だけを置き換える。前後の文字はそのまま見える。
(ert-deftest peta-journal-an-image-replaces-only-its-link ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "前![p](../assets/p.png)後"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (let* ((start (peta-journal--find-row "b"))
           (overlay (seq-find (lambda (overlay) (overlay-get overlay 'display))
                              (overlays-in start (peta-journal--body-end start)))))
      (should (equal (buffer-substring-no-properties (overlay-start overlay) (overlay-end overlay))
                     "![p](../assets/p.png)")))))

;; リンクの文字に、`\`を付けた`[`と`]`や、対になった`[]`があっても、画像が見える（ADR 022）。
(ert-deftest peta-journal-an-image-shows-when-its-link-text-has-brackets ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![図\\[1\\]\\\\](../assets/p.png)")
                                        (peta-journal-test--row "c" 1 "![図 [2] 版](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (should (peta-journal-test--image-of "b"))
    (should (peta-journal-test--image-of "c"))))

;; `]`で終わったリンクの文字の後ろは、画像のリンクではない。
(ert-deftest peta-journal-a-link-text-ends-at-its-unpaired-bracket ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![図]1](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (should-not (peta-journal-test--image-of "b"))))

;; 画像の種類を表示できないEmacsでは、リンクの文字のまま見え、エラーにならない。
(ert-deftest peta-journal-an-image-of-a-type-emacs-cannot-show-stays-a-link ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    ;; A build without PNG, such as Emacs 31.1 on a Mac in 2026-11
    (let ((image-types (remq 'png image-types)))
      (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png))
    (should-not (peta-journal-test--image-of "b"))
    (should (equal (peta-journal-test--shown) "• 上\n• ![p](../assets/p.png)\n"))))

;; サーバーの外にある画像（`http://`と`https://`）も、そのURLから取得して見える。`#`のあとは大きさで、取得するURLには含めない。
(ert-deftest peta-journal-an-image-outside-the-server-shows-from-its-url ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](https://example.com/a/p.png#w=20)")
                                        (peta-journal-test--row "c" 1 "![q](http://example.com/q.png)"))
    (should (equal peta-journal-test--fetched
                   '("https://example.com/a/p.png" "http://example.com/q.png")))
    (peta-journal--image-arrived "https://example.com/a/p.png" peta-journal-test--png)
    (peta-journal--image-arrived "http://example.com/q.png" peta-journal-test--png)
    (should (equal (image-property (peta-journal-test--image-of "b") :width) 20))
    (should (peta-journal-test--image-of "c"))))

;; `http`でも`https`でもないURLと、サーバーのassetsでない相対パスの画像は、文字のままで、取りにも行かない。
(ert-deftest peta-journal-an-image-of-another-scheme-stays-a-link ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](file:///c:/p.png)")
                                        (peta-journal-test--row "c" 1 "![q](q.png)"))
    (should-not peta-journal-test--fetched)))

;; 画像の中身が上限を超えると、開いたページにない画像から、使ってから長いものの順に忘れる。
(ert-deftest peta-journal-images-over-the-limit-are-forgotten-from-the-least-used ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (let ((peta-journal-image-memory-limit (* 2 (length peta-journal-test--png))))
      (peta-journal--image-arrived (peta-journal-test--asset-url "old.png") peta-journal-test--png)
      (peta-journal--image-arrived (peta-journal-test--asset-url "new.png") peta-journal-test--png)
      (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
      (should-not (gethash (peta-journal-test--asset-url "old.png") peta-journal--images))
      (should (gethash (peta-journal-test--asset-url "new.png") peta-journal--images))
      (should (peta-journal-test--image-of "b")))))

;; 開いたページにある画像は、上限を超えても忘れない。忘れると、取り直しと描き直しを繰り返す。
(ert-deftest peta-journal-an-image-on-an-open-page-is-kept-over-the-limit ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "c" 1 "![q](../assets/q.png)"))
    (let ((peta-journal-image-memory-limit 1))
      ;; The page asked for both as it was drawn
      (setq peta-journal-test--fetched nil)
      (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
      (peta-journal--image-arrived (peta-journal-test--asset-url "q.png") peta-journal-test--png)
      (should (peta-journal-test--image-of "b"))
      (should (peta-journal-test--image-of "c"))
      (should-not peta-journal-test--fetched))))

;; カーソルのある項目では、画像ではなくリンクの文字が見える。
(ert-deftest peta-journal-the-item-with-the-point-shows-its-link-as-text ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "b" 1 "下"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (should-not (peta-journal-test--image-of "a"))))

;; カーソルを動かすと、入った項目は文字になり、出た項目は画像に戻る。
(ert-deftest peta-journal-moving-the-point-swaps-the-image-and-the-text ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "b" 1 "![q](../assets/q.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal--image-arrived (peta-journal-test--asset-url "q.png") peta-journal-test--png)
    (should (peta-journal-test--image-of "b"))
    (peta-journal-test--goto "b" 0)
    (should (peta-journal-test--image-of "a"))
    (should-not (peta-journal-test--image-of "b"))
    (peta-journal-test--goto "a" 0)
    (should-not (peta-journal-test--image-of "a"))
    (should (peta-journal-test--image-of "b"))))

;; カーソルを動かしても、本文の変更としてシェルへは送られない。
(ert-deftest peta-journal-swapping-an-image-sends-no-change-of-the-body ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "b" 1 "下"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal-test--goto "b" 0)
    (peta-journal-test--goto "a" 0)
    (should-not (peta-journal-test--sent-of "body_changed"))))

;; 項目を出ると、そこで打ったリンクも画像になる。
(ert-deftest peta-journal-a-typed-link-becomes-an-image-when-the-point-leaves ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "")
                                        (peta-journal-test--row "b" 1 "下"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal-test--goto "a" 0)
    (insert "![p](../assets/p.png)")
    (peta-journal--post-command)
    (should-not (peta-journal-test--image-of "a"))
    (peta-journal-test--goto "b" 0)
    (should (peta-journal-test--image-of "a"))))

;; シェルが項目を書き直しても、新しい本文の画像が見える。リンクが消えれば画像も消える。
(ert-deftest peta-journal-a-rewritten-item-shows-the-image-of-its-new-body ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "下"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "上")
                                      (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
                              "page" 0)
    (should (peta-journal-test--image-of "b"))
    (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "上")
                                      (peta-journal-test--row "b" 1 "文字だけ"))
                              "page" 0)
    (should-not (peta-journal-test--image-of "b"))))

;; シェルの指示でカーソルが別の項目へ移っても、文字と画像が入れ替わる。
(ert-deftest peta-journal-a-focus-from-the-shell-swaps-the-image-and-the-text ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "b" 1 "下"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal--on-focus "b" 0 peta-journal--seq)
    (should (peta-journal-test--image-of "a"))))

;; 画像の中身は1回だけサーバーへ取りに行く。同じ画像が2つあっても、カーソルが動いても同じである。
(ert-deftest peta-journal-an-image-is-fetched-once ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)")
                                        (peta-journal-test--row "c" 1 "![p](../assets/p.png)"))
    (peta-journal-test--goto "b" 0)
    (peta-journal-test--goto "a" 0)
    (should (equal peta-journal-test--fetched
                   (list (peta-journal-test--asset-url "p.png"))))))

;; 画像が届くまでは、リンクの文字が見える。
(ert-deftest peta-journal-a-link-stays-text-until-its-image-arrives ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (should-not (peta-journal-test--image-of "b"))))

(defun peta-journal-test--image-of-body (body &optional level)
  "The image shown for an item with BODY at LEVEL, below an item with the point."
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" (or level 1) body))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (peta-journal-test--image-of "b")))

;; リンクの#w=と#h=は、Web版と同じように画像の大きさになる。
(ert-deftest peta-journal-a-link-sets-the-size-of-its-image ()
  (let ((image (peta-journal-test--image-of-body "![p](../assets/p.png#w=300&h=240)")))
    (should (equal (image-property image :width) 300))
    (should (equal (image-property image :height) 240))))

;; 片方だけの指定は、その向きだけを決める。
(ert-deftest peta-journal-a-link-may-set-one-side-of-its-image ()
  (let ((image (peta-journal-test--image-of-body "![p](../assets/p.png#h=240)")))
    (should-not (image-property image :width))
    (should (equal (image-property image :height) 240))))

;; 正の整数でない指定は、Web版と同じように無視する。
(ert-deftest peta-journal-a-bad-size-in-a-link-is-ignored ()
  (let ((image (peta-journal-test--image-of-body "![p](../assets/p.png#w&h=big)")))
    (should image)
    (should-not (image-property image :width))
    (should-not (image-property image :height)))
  (should-not (image-property (peta-journal-test--image-of-body "![p](../assets/p.png#w=0)")
                              :width)))

;; 画像はウィンドウに収まる。字下げの深い項目では、字下げの分だけ小さくなる。
(ert-deftest peta-journal-an-image-fits-the-room-beside-its-indent ()
  (cl-letf (((symbol-function 'window-body-width) (lambda (&rest _) 800))
            ((symbol-function 'window-body-height) (lambda (&rest _) 600)))
    (let ((shallow (peta-journal-test--image-of-body "![p](../assets/p.png)" 1))
          (deep (peta-journal-test--image-of-body "![p](../assets/p.png)" 3)))
      (should (<= (image-property shallow :max-width) 800))
      (should (<= (image-property shallow :max-height) 600))
      (should (= (- (image-property shallow :max-width) (image-property deep :max-width))
                 (* 4 (frame-char-width)))))))

;; ウィンドウの大きさが変わると、画像も合わせ直す。
(ert-deftest peta-journal-an-image-follows-the-size-of-the-window ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") peta-journal-test--png)
    (let ((before (image-property (peta-journal-test--image-of "b") :max-width)))
      (cl-letf (((symbol-function 'window-body-width) (lambda (&rest _) (+ before 100))))
        (peta-journal--on-window-size-change (selected-window)))
      (should (> (image-property (peta-journal-test--image-of "b") :max-width) before)))))

;; 取得できなかった画像はリンクの文字のままで、カーソルが動くたびに取りに行くことはない。
(ert-deftest peta-journal-a-missing-image-stays-text-and-is-not-fetched-again ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") nil)
    (peta-journal-test--goto "b" 0)
    (peta-journal-test--goto "a" 0)
    (should-not (peta-journal-test--image-of "b"))
    (should (= (length peta-journal-test--fetched) 1))))

;; 別のページを開くと、取得できなかった画像をもう一度取りに行く。サーバーが止まっていただけかもしれない。
(ert-deftest peta-journal-a-new-page-fetches-a-missing-image-again ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") nil)
    (peta-journal--apply-rows (vector (peta-journal-test--row "c" 1 "上")
                                      (peta-journal-test--row "d" 1 "![p](../assets/p.png)"))
                              "other-page" 0)
    (should (= (length peta-journal-test--fetched) 2))))

;; Emacsが読めない形式のファイルは、リンクの文字のままである。
(ert-deftest peta-journal-a-file-emacs-cannot-read-stays-text ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
    (peta-journal--image-arrived (peta-journal-test--asset-url "p.png") "not an image")
    (should-not (peta-journal-test--image-of "b"))))

;; 画像を出せない環境では、リンクの文字のままで、取りにも行かない。
(ert-deftest peta-journal-a-display-without-images-shows-the-text ()
  (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上"))
    (cl-letf (((symbol-function 'display-images-p) #'ignore))
      (peta-journal--apply-rows (vector (peta-journal-test--row "a" 1 "上")
                                        (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
                                "page" 0)
      (should-not (peta-journal-test--image-of "b"))
      (should-not peta-journal-test--fetched))))

;; コードの中のリンクは、書き方の例である。画像にしない。
(ert-deftest peta-journal-a-link-in-code-stays-text ()
  (dolist (body '("`![p](../assets/p.png)`" "```\n![p](../assets/p.png)\n```"))
    (should-not (peta-journal-test--image-of-body body))))

;; サーバーのURLの末尾に/があっても、画像のURLは正しい形になる。
(ert-deftest peta-journal-the-url-of-an-image-allows-a-slash-after-the-server ()
  (let ((peta-journal-server "http://localhost:9000/"))
    (peta-journal-test--with-images (list (peta-journal-test--row "a" 1 "上")
                                          (peta-journal-test--row "b" 1 "![p](../assets/p.png)"))
      (should (equal peta-journal-test--fetched '("http://localhost:9000/assets/p.png"))))))

;; 接続を始められなくても、エラーで止まらず、文字のままである。
(ert-deftest peta-journal-a-fetch-that-cannot-start-leaves-the-text ()
  (cl-letf (((symbol-function 'url-retrieve) (lambda (&rest _) (error "No connection"))))
    (peta-journal--fetch-image "http://localhost:1/assets/p.png")
    (should (eq (gethash "http://localhost:1/assets/p.png" peta-journal--images) 'missing))))

(defun peta-journal-test--offered-pages (command)
  "The pages COMMAND offers to open, in the order the completion list shows them.
Without a sort function of its own, the list is sorted by name, as `completions-sort' does."
  (let (offered)
    (cl-letf (((symbol-function 'completing-read)
               (lambda (_prompt collection &rest _)
                 (setq offered collection)
                 "")))
      (advice-eval-interactive-spec (cadr (interactive-form command))))
    (let ((sort (or (completion-metadata-get (completion-metadata "" offered nil)
                                             'display-sort-function)
                    (lambda (pages) (sort pages #'string<)))))
      (funcall sort (all-completions "" offered)))))

;; 新しいバッファに開くときの候補は、名前順に並べ替えず、最近のページの順のまま見せる。
(ert-deftest peta-journal-the-recent-pages-are-offered-in-their-own-order ()
  (let ((peta-journal--recent '("zeta" "alpha" "notes/mid")))
    (should (equal (peta-journal-test--offered-pages #'peta-journal-new-buffer)
                   '("zeta" "alpha" "notes/mid")))))

;; キーは、書きながら押すものを`C-c C-文字`に、ページ単位でたまに使うものを`C-c C-x`の下に置く。
;; 項目の移動は`M-S-矢印`にし、`M-矢印`はEmacsの単語の移動などに残す。
(ert-deftest peta-journal-the-keys-of-the-mode ()
  (pcase-dolist (`(,key . ,command)
                 '(("M-S-<up>" . peta-journal-move-up)
                   ("M-S-<down>" . peta-journal-move-down)
                   ("M-S-<right>" . peta-journal-indent)
                   ("M-S-<left>" . peta-journal-outdent)
                   ("C-c C-f" . peta-journal-collapse-toggle)
                   ("C-c C-s" . peta-journal-search)
                   ("C-c C-b" . peta-journal-back)
                   ("C-c C-x f" . peta-journal-forward)
                   ("C-c C-x b" . peta-journal-backlinks)
                   ("C-c C-x r" . peta-journal-rename-page)
                   ("C-c C-x s" . peta-journal-send-to-stash)
                   ("C-c C-x d" . peta-journal-delete-page)
                   ("C-c C-x p" . peta-journal-properties)
                   ("C-c C-x n" . peta-journal-new-buffer)
                   ("C-c C-x i" . peta-journal-insert-image)
                   ("C-c C-x c" . peta-journal-calendar)
                   ;; `text-mode`では、Emacsの版によってスペルの補完になるキーである。
                   ("C-M-i" . completion-at-point)))
    (should (eq (keymap-lookup peta-journal-mode-map key) command)))
  ;; `C-c C-c`はEmacsでは「確定」の意味なので、折り畳みには使わずに空けておく。
  (dolist (key '("M-<up>" "M-<down>" "M-<right>" "M-<left>" "C-c /" "C-c C-c"
                 "C-c C-l" "C-c C-r" "C-c C-a" "C-c C-v" "C-c C-i" "C-c C-k"))
    (should-not (keymap-lookup peta-journal-mode-map key))))

;;;; The calendar

(defmacro peta-journal-test--with-calendar (date &rest body)
  "Run BODY in the calendar of a page, with the point on DATE.
DATE is a list of the month, the day and the year."
  (declare (indent 1))
  `(peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
     (unwind-protect
         (progn
           (peta-journal-calendar)
           (calendar-goto-date ,date)
           ,@body)
       (when (get-buffer calendar-buffer)
         (kill-buffer calendar-buffer)))))

;; カレンダーの日でRETを押すと、その日のジャーナルを開き、カレンダーは閉じる。
(ert-deftest peta-journal-ret-in-the-calendar-opens-the-journal-of-the-day ()
  (peta-journal-test--with-calendar '(2 10 2026)
    (should (eq major-mode 'calendar-mode))
    (funcall (key-binding (kbd "RET")))
    (should (equal (car (last peta-journal-test--sent))
                   '("go_to_date" :date "2026-02-10" :unit "day")))
    (should-not (get-buffer-window calendar-buffer))))

;; カレンダーを開くと、日、週、月、年を開くキーと、閉じるキーの案内が出る。
(ert-deftest peta-journal-opening-the-calendar-tells-its-keys ()
  (let (told)
    (cl-letf (((symbol-function 'message)
               (lambda (format-string &rest args)
                 (setq told (apply #'format-message format-string args)))))
      (peta-journal-test--with-calendar '(2 10 2026)
        (should (equal told "RET: day  w: week  m: month  y: year  q: close"))))))

;; カレンダーの日でw、m、yを押すと、その日の週、月、年のページを開く。
(ert-deftest peta-journal-w-m-and-y-in-the-calendar-open-the-week-the-month-and-the-year ()
  (dolist (case '(("w" . "week") ("m" . "month") ("y" . "year")))
    (peta-journal-test--with-calendar '(12 31 2026)
      (funcall (key-binding (kbd (car case))))
      (should (equal (car (last peta-journal-test--sent))
                     `("go_to_date" :date "2026-12-31" :unit ,(cdr case)))))))

;; カレンダーで選ぶ前に、編集中の項目の編集を終える。入力した文を保存するためである。
(ert-deftest peta-journal-a-date-of-the-calendar-ends-the-edit-first ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "x"))
    (unwind-protect
        (progn
          (peta-journal-test--goto "a" 1)
          (peta-journal-calendar)
          (calendar-goto-date '(2 10 2026))
          (setq peta-journal-test--sent nil)
          (peta-journal-calendar-open-day)
          (should (equal (mapcar #'car peta-journal-test--sent)
                         '("edit_finished" "go_to_date"))))
      (when (get-buffer calendar-buffer)
        (kill-buffer calendar-buffer)))))

;; 日のないところでは、何も開かず、そのことを伝える。
(ert-deftest peta-journal-a-place-of-the-calendar-without-a-date-opens-nothing ()
  (peta-journal-test--with-calendar '(2 10 2026)
    (goto-char (point-min))
    (setq peta-journal-test--sent nil)
    (should-error (peta-journal-calendar-open-day) :type 'user-error)
    (should-not peta-journal-test--sent)))

;; ふつうに開いたカレンダーには、PetaJournalのキーは付かない。
(ert-deftest peta-journal-a-calendar-opened-by-itself-has-no-keys-of-peta-journal ()
  (peta-journal-test--with-calendar '(2 10 2026)
    (calendar-exit)
    (calendar)
    (should-not (eq (key-binding (kbd "RET")) 'peta-journal-calendar-open-day))
    (should-not (eq (key-binding (kbd "w")) 'peta-journal-calendar-open-week))))

;;;; Completing a wiki link

(defmacro peta-journal-test--finding (names &rest body)
  "Run BODY with a shell that finds the page NAMES for whatever is asked."
  (declare (indent 1))
  `(let ((collect (symbol-function 'peta-journal--send)))
     (cl-letf (((symbol-function 'peta-journal--send)
                (lambda (type &rest fields)
                  (apply collect type fields)
                  (when (equal type "page_names")
                    (peta-journal--on-line (list :t "page_names"
                                                 :query (plist-get fields :query)
                                                 :titles (vconcat ,names)))))))
       (unwind-protect
           (progn ,@body)
         (completion-in-region-mode -1)
         (when (get-buffer "*Completions*")
           (kill-buffer "*Completions*"))))))

(defun peta-journal-test--offered-names ()
  "The page names the completion at the point offers, in the order the list shows them."
  (pcase-let* ((`(,start ,end ,table) (peta-journal--complete-wiki-link))
               (typed (buffer-substring-no-properties start end))
               (names (completion-all-completions typed table nil (- (point) start)))
               (sort (completion-metadata-get (completion-metadata typed table nil)
                                              'display-sort-function)))
    ;; The last cdr is the length of the text the names do not replace, not a name
    (when names
      (setcdr (last names) nil))
    (mapcar #'substring-no-properties (funcall sort names))))

;; `[[`と`]]`の間で補完すると、そこに打った文字でページ名を1回だけ尋ねる。
;; 見つかった名前が1つなら、打った文字がその名前に置き換わり、本文として送られる。
;; カーソルは`]]`の前に残る。
(ert-deftest peta-journal-completing-in-a-wiki-link-puts-in-the-page-name-that-was-found ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[To]] now"))
    (peta-journal-test--finding '("Tokyo")
      (peta-journal-test--goto "a" 8)
      (setq peta-journal-test--sent nil)
      (completion-at-point)
      (should (equal (peta-journal-test--sent-of "page_names")
                     '(("page_names" :id "a" :query "To"))))
      (should (equal (car (last (peta-journal-test--sent-of "body_changed")))
                     '("body_changed" :id "a" :body "see [[Tokyo]] now")))
      (should (looking-at-p "\\]\\] now")))))

;; カーソルが名前の途中にあっても、`[[`と`]]`の間の全体で尋ね、全体を置き換える。
(ert-deftest peta-journal-completing-in-the-middle-of-a-name-replaces-the-whole-name ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "[[kyo]]"))
    (peta-journal-test--finding '("Tokyo")
      (peta-journal-test--goto "a" 3)
      (setq peta-journal-test--sent nil)
      (completion-at-point)
      (should (equal (peta-journal-test--sent-of "page_names")
                     '(("page_names" :id "a" :query "kyo"))))
      (should (equal (peta-journal--body-at (point)) "[[Tokyo]]")))))

;; 候補は、サーバーが見つけた名前を、その順のまま示す。打った文字で始まらない名前も除かない。
(ert-deftest peta-journal-the-page-names-are-offered-as-the-server-found-them ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[kyo]]"))
    (peta-journal-test--finding '("Tokyo" "Kyoto" "notes/kyo")
      (peta-journal-test--goto "a" 9)
      (should (equal (peta-journal-test--offered-names) '("Tokyo" "Kyoto" "notes/kyo"))))))

;; 見つかった名前がいくつもあるときは、選ぶまで本文は変わらない。
(ert-deftest peta-journal-several-page-names-leave-the-body-until-one-is-chosen ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[To]]"))
    (peta-journal-test--finding '("Tokyo" "Tools")
      (peta-journal-test--goto "a" 8)
      (setq peta-journal-test--sent nil)
      (completion-at-point)
      (should-not (peta-journal-test--sent-of "body_changed"))
      (should (equal (peta-journal--body-at (point)) "see [[To]]")))))

;; 名前が見つからなければ、本文は変わらない。
(ert-deftest peta-journal-no-page-name-found-leaves-the-body ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[To]]"))
    (peta-journal-test--finding nil
      (peta-journal-test--goto "a" 8)
      (setq peta-journal-test--sent nil)
      (completion-at-point)
      (should-not (peta-journal-test--sent-of "body_changed"))
      (should (equal (peta-journal--body-at (point)) "see [[To]]")))))

;; 何も打っていなければ、サーバーには尋ねず、最近のページをその順のまま示す。
(ert-deftest peta-journal-an-empty-wiki-link-is-completed-with-the-recent-pages ()
  (let ((peta-journal--recent '("zeta" "alpha")))
    (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[]]"))
      (peta-journal-test--finding '("Tokyo")
        (peta-journal-test--goto "a" 6)
        (should (equal (peta-journal-test--offered-names) '("zeta" "alpha")))
        (should-not (peta-journal-test--sent-of "page_names"))))))

;; ウィキリンクの外では、ページ名を補完しない。
;; 閉じた`]]`の後ろ、`[[`や`]]`の2文字の間、`[`が1つだけのところも外である。
(ert-deftest peta-journal-outside-a-wiki-link-no-page-name-is-completed ()
  (peta-journal-test--with-rows (list (peta-journal-test--row "a" 1 "see [[To]] [x"))
    (peta-journal-test--finding '("Tokyo")
      (dolist (cursor '(2 5 9 10 13))
        (peta-journal-test--goto "a" cursor)
        (should-not (peta-journal--complete-wiki-link)))
      (should-not (peta-journal-test--sent-of "page_names")))))

;;; peta-journal-test.el ends here
