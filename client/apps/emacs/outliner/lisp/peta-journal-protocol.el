;;; peta-journal-protocol.el --- The lines of the shell -*- lexical-binding: t; -*-

;;; Commentary:

;; Written from the Rust types of src/protocol.rs (ADR 078).  Do not edit it by hand.
;; Write it again with:
;;   PETA_WRITE_LISP=1 cargo test -p outliner-emacs the_committed_lisp_protocol

;;; Code:

(defsubst peta-journal-protocol-backlink-entry-title (value)
  "The field title of the type backlink-entry."
  (plist-get value :title))

(defsubst peta-journal-protocol-backlink-entry-url (value)
  "The field url of the type backlink-entry."
  (plist-get value :url))

(defsubst peta-journal-protocol-backlink-entry-items (value)
  "The field items of the type backlink-entry."
  (plist-get value :items))

(defsubst peta-journal-protocol-backlink-line-text (value)
  "The field text of the type backlink-line."
  (plist-get value :text))

(defsubst peta-journal-protocol-backlink-line-url (value)
  "The field url of the type backlink-line."
  (plist-get value :url))

(defconst peta-journal-protocol-bullet-names '("plain" "transclusion" "broken" "hidden")
  "What the mark at the start of a row shows. A task has the bullet of its
kind too: its state is a mark of its own, as in the web frontend.")

(defconst peta-journal-protocol-emacs-look-names '("bold" "italic" "struck" "underlined" "marked" "code" "raised" "lowered" "hidden" "heading1" "heading2" "heading3" "heading4" "heading5" "heading6" "mark" "quote" "rule" "code_block" "table_header")
  "How a part of the text looks.")

(defsubst peta-journal-protocol-emacs-span-from (value)
  "Places in `text`, in characters (code points, ADR 043)."
  (plist-get value :from))

(defsubst peta-journal-protocol-emacs-span-to (value)
  "The field to of the type emacs-span."
  (plist-get value :to))

(defsubst peta-journal-protocol-emacs-span-looks (value)
  "The field looks of the type emacs-span."
  (plist-get value :looks))

(defsubst peta-journal-protocol-emacs-span-link (value)
  "The field link of the type emacs-span."
  (plist-get value :link))

(defsubst peta-journal-protocol-emacs-span-image (value)
  "An image of the assets folder, shown in place of the text of the span."
  (plist-get value :image))

(defsubst peta-journal-protocol-emacs-span-letters (value)
  "False for what Rust added, such as a list mark or the spaces of a table:
they are not letters of the shown text (ADR 044)."
  (plist-get value :letters))

(defsubst peta-journal-protocol-emacs-text-text (value)
  "The shown text. A line break after each paragraph and each line of code."
  (plist-get value :text))

(defsubst peta-journal-protocol-emacs-text-spans (value)
  "The parts of the text that are not plain shown letters, in the order of
`from`."
  (plist-get value :spans))

(defsubst peta-journal-protocol-free-key-key (value)
  "The field key of the type free-key."
  (plist-get value :key))

(defsubst peta-journal-protocol-free-key-label (value)
  "The field label of the type free-key."
  (plist-get value :label))

(defconst peta-journal-protocol-outbound-fields
  '((:state "ack" "holding") (:rows "ack" "page_id" "changes") (:focus "ack" "id" "cursor") (:scroll "id") (:title "title") (:page-path "path") (:index "folder" "demote_name") (:properties "rows" "free_keys" "datetime_keys") (:status "connection") (:search "results" "searching") (:recent "paths") (:sidebar "index" "headings" "recent") (:backlinks "groups") (:message "text") (:copy "text") (:browse "address") (:page-names "query" "titles") (:ask "req" "message"))
  "The fields of each kind of the type outbound, in the order its handler
takes them.")

(defun peta-journal-protocol-handle-outbound (value handlers)
  "Call the function in HANDLERS for the kind of VALUE, with its fields.
HANDLERS is a plist from the keyword of each kind to its function.
The fields come in the order of `peta-journal-protocol-outbound-fields'."
  (pcase (plist-get value :t)
    ("state"
     (funcall (plist-get handlers :state)
              (plist-get value :ack) (plist-get value :holding)))
    ("rows"
     (funcall (plist-get handlers :rows)
              (plist-get value :ack) (plist-get value :page_id) (plist-get value :changes)))
    ("focus"
     (funcall (plist-get handlers :focus)
              (plist-get value :ack) (plist-get value :id) (plist-get value :cursor)))
    ("scroll"
     (funcall (plist-get handlers :scroll)
              (plist-get value :id)))
    ("title"
     (funcall (plist-get handlers :title)
              (plist-get value :title)))
    ("page_path"
     (funcall (plist-get handlers :page-path)
              (plist-get value :path)))
    ("index"
     (funcall (plist-get handlers :index)
              (plist-get value :folder) (plist-get value :demote_name)))
    ("properties"
     (funcall (plist-get handlers :properties)
              (plist-get value :rows) (plist-get value :free_keys) (plist-get value :datetime_keys)))
    ("status"
     (funcall (plist-get handlers :status)
              (plist-get value :connection)))
    ("search"
     (funcall (plist-get handlers :search)
              (plist-get value :results) (plist-get value :searching)))
    ("recent"
     (funcall (plist-get handlers :recent)
              (plist-get value :paths)))
    ("sidebar"
     (funcall (plist-get handlers :sidebar)
              (plist-get value :index) (plist-get value :headings) (plist-get value :recent)))
    ("backlinks"
     (funcall (plist-get handlers :backlinks)
              (plist-get value :groups)))
    ("message"
     (funcall (plist-get handlers :message)
              (plist-get value :text)))
    ("copy"
     (funcall (plist-get handlers :copy)
              (plist-get value :text)))
    ("browse"
     (funcall (plist-get handlers :browse)
              (plist-get value :address)))
    ("page_names"
     (funcall (plist-get handlers :page-names)
              (plist-get value :query) (plist-get value :titles)))
    ("ask"
     (funcall (plist-get handlers :ask)
              (plist-get value :req) (plist-get value :message)))))

(defsubst peta-journal-protocol-property-row-key (value)
  "The field key of the type property-row."
  (plist-get value :key))

(defsubst peta-journal-protocol-property-row-label (value)
  "What the list shows for the key: the body of its name and its type, as
`rating (number)` (ADR 047)."
  (plist-get value :label))

(defsubst peta-journal-protocol-property-row-value (value)
  "The field value of the type property-row."
  (plist-get value :value))

(defsubst peta-journal-protocol-property-row-removable (value)
  "The field removable of the type property-row."
  (plist-get value :removable))

(defsubst peta-journal-protocol-property-row-editable (value)
  "The field editable of the type property-row."
  (plist-get value :editable))

(defsubst peta-journal-protocol-rich-image-path (value)
  "Where its file is under the Markdown folder, such as
`assets/picture.png`."
  (plist-get value :path))

(defsubst peta-journal-protocol-rich-image-alt (value)
  "What stands for the image where it cannot be seen."
  (plist-get value :alt))

(defsubst peta-journal-protocol-rich-image-width (value)
  "The width that the page asks for, in points. `None` when it asks for
none."
  (plist-get value :width))

(defsubst peta-journal-protocol-rich-image-height (value)
  "The height that the page asks for, the same way."
  (plist-get value :height))

(defconst peta-journal-protocol-rich-link-fields
  '((:page "url") (:web "url") (:file "path"))
  "The fields of each kind of the type rich-link, in the order its handler
takes them.")

(defun peta-journal-protocol-handle-rich-link (value handlers)
  "Call the function in HANDLERS for the kind of VALUE, with its fields.
HANDLERS is a plist from the keyword of each kind to its function.
The fields come in the order of `peta-journal-protocol-rich-link-fields'."
  (pcase value
    (`(:Page ,fields)
     (funcall (plist-get handlers :page)
              (plist-get fields :url)))
    (`(:Web ,fields)
     (funcall (plist-get handlers :web)
              (plist-get fields :url)))
    (`(:File ,fields)
     (funcall (plist-get handlers :file)
              (plist-get fields :path)))))

(defconst peta-journal-protocol-row-change-fields
  '((:insert "at" "item") (:remove "at") (:move "from" "to") (:update "at" "item") (:reset "items"))
  "The fields of each kind of the type row-change, in the order its handler
takes them.")

(defun peta-journal-protocol-handle-row-change (value handlers)
  "Call the function in HANDLERS for the kind of VALUE, with its fields.
HANDLERS is a plist from the keyword of each kind to its function.
The fields come in the order of `peta-journal-protocol-row-change-fields'."
  (pcase value
    (`(:Insert ,fields)
     (funcall (plist-get handlers :insert)
              (plist-get fields :at) (plist-get fields :item)))
    (`(:Remove ,fields)
     (funcall (plist-get handlers :remove)
              (plist-get fields :at)))
    (`(:Move ,fields)
     (funcall (plist-get handlers :move)
              (plist-get fields :from) (plist-get fields :to)))
    (`(:Update ,fields)
     (funcall (plist-get handlers :update)
              (plist-get fields :at) (plist-get fields :item)))
    (`(:Reset ,fields)
     (funcall (plist-get handlers :reset)
              (plist-get fields :items)))))

(defsubst peta-journal-protocol-row-view-id (value)
  "The field id of the type row-view."
  (plist-get value :id))

(defsubst peta-journal-protocol-row-view-level (value)
  "The field level of the type row-view."
  (plist-get value :level))

(defsubst peta-journal-protocol-row-view-body (value)
  "The Markdown of the body, which Emacs shows while the item is edited (ADR
077)."
  (plist-get value :body))

(defsubst peta-journal-protocol-row-view-text (value)
  "The body as Emacs shows it while the item is not edited."
  (plist-get value :text))

(defsubst peta-journal-protocol-row-view-bullet (value)
  "The field bullet of the type row-view."
  (plist-get value :bullet))

(defsubst peta-journal-protocol-row-view-task (value)
  "The state of the task, which Emacs marks behind the bullet. None when the
item is not a task."
  (plist-get value :task))

(defsubst peta-journal-protocol-row-view-collapsed (value)
  "The field collapsed of the type row-view."
  (plist-get value :collapsed))

(defsubst peta-journal-protocol-row-view-header (value)
  "The date above the first item of a journal day."
  (plist-get value :header))

(defsubst peta-journal-protocol-row-view-selected (value)
  "The field selected of the type row-view."
  (plist-get value :selected))

(defsubst peta-journal-protocol-search-entry-position (value)
  "The field position of the type search-entry."
  (plist-get value :position))

(defsubst peta-journal-protocol-search-entry-kind (value)
  "The field kind of the type search-entry."
  (plist-get value :kind))

(defsubst peta-journal-protocol-search-entry-title (value)
  "The path of the page, or of the page to create."
  (plist-get value :title))

(defsubst peta-journal-protocol-search-entry-label (value)
  "The field label of the type search-entry."
  (plist-get value :label))

(defsubst peta-journal-protocol-search-entry-snippet (value)
  "The field snippet of the type search-entry."
  (plist-get value :snippet))

(defconst peta-journal-protocol-search-kind-names '("today" "page" "create" "item")
  "What a line of the search results opens.")

(defsubst peta-journal-protocol-sidebar-heading-id (value)
  "The field id of the type sidebar-heading."
  (plist-get value :id))

(defsubst peta-journal-protocol-sidebar-heading-text (value)
  "The field text of the type sidebar-heading."
  (plist-get value :text))

(defsubst peta-journal-protocol-sidebar-heading-level (value)
  "The field level of the type sidebar-heading."
  (plist-get value :level))

(defsubst peta-journal-protocol-sidebar-index-header (value)
  "The folder, as `a/b/`, or `(root)/`."
  (plist-get value :header))

(defsubst peta-journal-protocol-sidebar-index-path (value)
  "The path of the index page that the header opens."
  (plist-get value :path))

(defsubst peta-journal-protocol-sidebar-index-parent (value)
  "The path that the `../` line opens. `None` in the root."
  (plist-get value :parent))

(defsubst peta-journal-protocol-sidebar-index-pages (value)
  "The field pages of the type sidebar-index."
  (plist-get value :pages))

(defsubst peta-journal-protocol-sidebar-line-label (value)
  "The field label of the type sidebar-line."
  (plist-get value :label))

(defsubst peta-journal-protocol-sidebar-line-dir (value)
  "The folder shown before the label, for a recent page in a folder."
  (plist-get value :dir))

(defsubst peta-journal-protocol-sidebar-line-path (value)
  "The field path of the type sidebar-line."
  (plist-get value :path))

(defsubst peta-journal-protocol-sidebar-line-folder (value)
  "The field folder of the type sidebar-line."
  (plist-get value :folder))

(defsubst peta-journal-protocol-sidebar-line-current (value)
  "The page is the one on screen."
  (plist-get value :current))

(defsubst peta-journal-protocol-sidebar-view-index (value)
  "The pages of the folder of the shown page. `None` when there is nothing
to list."
  (plist-get value :index))

(defsubst peta-journal-protocol-sidebar-view-headings (value)
  "The field headings of the type sidebar-view."
  (plist-get value :headings))

(defsubst peta-journal-protocol-sidebar-view-recent (value)
  "The field recent of the type sidebar-view."
  (plist-get value :recent))

(defconst peta-journal-protocol-task-mark-names '("task" "running" "done")
  "The state of a task, for the mark behind the bullet.")

(provide 'peta-journal-protocol)
;;; peta-journal-protocol.el ends here
