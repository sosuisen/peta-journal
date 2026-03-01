import { Component, input, output, ElementRef, viewChild, AfterViewInit, inject, effect, OnDestroy } from '@angular/core';
import { OutlineStateService } from '../../core/services/outline-state.service';

export interface WikiLinkContext {
  query: string;
  rect: DOMRect;
}

export function hasUnclosedCodeFence(text: string): boolean {
  let count = 0;
  for (const line of text.split('\n')) {
    if (line.trimStart().startsWith('```')) count++;
  }
  return count % 2 !== 0;
}

export function ensureCodeFenceClosed(text: string): string {
  return hasUnclosedCodeFence(text) ? text + '\n```' : text;
}


@Component({
  selector: 'app-item-editor',
  template: `
    @if (status() !== 'normal') {
      <input
        #checkbox
        type="checkbox"
        class="task-checkbox"
        [checked]="status() === 'done'"
        (click)="onCheckboxClick($event)"
      />
    }
    <textarea
      #editor
      [id]="'item-editor-' + itemId()"
      [value]="body()"
      (input)="onTextInput($event)"
      (keydown)="onKeydown($event)"
      (paste)="onPaste($event)"
      (blur)="blurred.emit()"
      (mousedown)="onEditorMouseDown($event)"
      rows="1"
    ></textarea>
  `,
  styles: [`
    :host {
      display: flex;
      align-items: flex-start;
      flex: 1;
      min-width: 0;
    }
    textarea {
      flex: 1;
      min-width: 0;
      border: none;
      outline: none;
      resize: none;
      font: inherit;
      line-height: 1.5;
      padding: 2px 0;
      background: transparent;
      overflow: hidden;
      font-family: inherit;
    }
    .task-checkbox {
      cursor: pointer;
      margin-right: 6px;
      margin-top: 4px;
      flex-shrink: 0;
    }
  `]
})
export class ItemEditorComponent implements AfterViewInit, OnDestroy {
  private readonly state = inject(OutlineStateService);
  readonly body = input.required<string>();
  readonly itemId = input.required<string>();
  readonly status = input.required<string>();

  readonly bodyChange = output<string>();
  readonly blurred = output<void>();
  readonly enterPressed = output<{ cursorPos: number }>();
  readonly backspaceAtStart = output<void>();
  readonly deleteAtEnd = output<void>();
  readonly navigateUp = output<number>();
  readonly navigateDown = output<number>();
  readonly indentRequested = output<void>();
  readonly outdentRequested = output<void>();
  readonly moveUp = output<void>();
  readonly moveDown = output<void>();
  readonly statusCycleRequested = output<void>();
  readonly statusToggled = output<void>();
  readonly pasteDetected = output<{ text: string; html: string; cursorPos: number }>();
  readonly imagePasteDetected = output<{ file: File; cursorPos: number }>();
  readonly dragBeyondBounds = output<'above' | 'below'>();
  readonly wikiLinkQuery = output<WikiLinkContext | null>();
  readonly wikiLinkSelect = output<string>(); // emits the key: 'ArrowUp', 'ArrowDown', 'Enter'
  readonly openWikiLink = output<string>();
  readonly shiftSelectUp = output<void>();
  readonly shiftSelectDown = output<void>();
  readonly collapseToggled = output<void>();
  readonly undoRequested = output<void>();
  readonly redoRequested = output<void>();
  readonly suggestionsVisible = input<boolean>(false);

  private readonly editorRef = viewChild.required<ElementRef<HTMLTextAreaElement>>('editor');
  private readonly checkboxRef = viewChild<ElementRef<HTMLInputElement>>('checkbox');

  private textareaRect: DOMRect | null = null;
  private isDragging = false;
  private dragSkipSelectionCheck = false;
  private mouseMoveHandler = (e: MouseEvent) => this.onDocMouseMove(e);
  private mouseUpHandler = () => this.onDocMouseUp();

  constructor() {
    effect(() => {
      const checkbox = this.checkboxRef()?.nativeElement;
      if (checkbox) {
        checkbox.indeterminate = this.status() === 'running';
      }
    });
  }

  /**
   * Initializes the editor after the view is rendered.
   * Sets cursor position, focuses the textarea, auto-resizes it,
   * and starts drag tracking if the edit was triggered by a mouse press.
   */
  ngAfterViewInit(): void {
    const el = this.editorRef().nativeElement;
    const pendingPos = this.state.consumePendingCursorPosition();
    if (pendingPos !== null) {
      const pos = Math.min(pendingPos, el.value.length);
      el.setSelectionRange(pos, pos);
    }
    el.focus();
    this.autoResize(el);

    if (this.state.consumePendingMouseDrag()) {
      this.textareaRect = el.getBoundingClientRect();
      this.isDragging = true;
      this.dragSkipSelectionCheck = true;
      document.addEventListener('mousemove', this.mouseMoveHandler);
      document.addEventListener('mouseup', this.mouseUpHandler);
    }
  }

  /**
   * Sets the cursor to a specific position within the textarea and focuses it.
   * @param pos - The character index to place the cursor at.
   */
  setCursorPosition(pos: number): void {
    const el = this.editorRef().nativeElement;
    el.setSelectionRange(pos, pos);
    el.focus();
  }

  /**
   * Moves the cursor to the end of the textarea content and focuses it.
   */
  setCursorToEnd(): void {
    const el = this.editorRef().nativeElement;
    const len = el.value.length;
    el.setSelectionRange(len, len);
    el.focus();
  }

  /**
   * Handles checkbox click by preventing the default toggle and emitting a status toggle event.
   * @param event - The mouse event from the checkbox click.
   */
  onCheckboxClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.statusToggled.emit();
  }

  /**
   * Handles text input in the textarea, emitting the new value
   * and checking for wiki link context.
   * @param event - The input event from the textarea.
   */
  onTextInput(event: Event): void {
    const inputEvent = event as InputEvent;
    const el = event.target as HTMLTextAreaElement;

    if (inputEvent.inputType === 'insertText' && inputEvent.data === '`') {
      this.autoCloseCodeFence(el);
    }

    this.bodyChange.emit(el.value);
    this.autoResize(el);
    this.checkWikiLinkContext(el);
  }

  private autoCloseCodeFence(el: HTMLTextAreaElement): void {
    const { value, selectionStart } = el;
    const lineStart = value.lastIndexOf('\n', selectionStart - 1) + 1;
    const currentLine = value.substring(lineStart, selectionStart);
    if (!currentLine.trimStart().startsWith('```')) return;
    if (hasUnclosedCodeFence(value)) {
      const newValue = value + '\n```';
      el.value = newValue;
      el.setSelectionRange(selectionStart, selectionStart);
    }
  }

  /**
   * Handles keydown events in the textarea, dispatching keyboard shortcuts
   * for navigation, editing, indentation, status cycling, and wiki link interaction.
   * @param event - The keyboard event from the textarea.
   */
  onKeydown(event: KeyboardEvent): void {
    const el = event.target as HTMLTextAreaElement;
    const { selectionStart, selectionEnd, value } = el;

    // When wiki link suggestions are visible, intercept navigation keys
    if (this.suggestionsVisible()) {
      if (event.key === 'ArrowUp' || event.key === 'ArrowDown' || event.key === 'Enter' || event.key === 'Escape') {
        event.preventDefault();
        if (event.key === 'Escape') {
          this.wikiLinkQuery.emit(null);
        } else {
          this.wikiLinkSelect.emit(event.key);
        }
        return;
      }
    }

    // Ctrl+Z / Ctrl+Shift+Z: undo/redo
    if ((event.key === 'z' || event.key === 'Z') && event.ctrlKey && !event.altKey) {
      event.preventDefault();
      if (event.shiftKey) {
        this.redoRequested.emit();
      } else {
        this.undoRequested.emit();
      }
      return;
    }

    // Ctrl+O: open wiki link at caret
    if (event.key === 'o' && event.ctrlKey) {
      const before = value.substring(0, selectionStart);
      const openIdx = before.lastIndexOf('[[');
      if (openIdx !== -1) {
        const between = before.substring(openIdx + 2);
        if (!between.includes(']]')) {
          const after = value.substring(selectionStart);
          const closeIdx = after.indexOf(']]');
          if (closeIdx !== -1) {
            const linkText = (between + after.substring(0, closeIdx)).trim();
            if (linkText) {
              event.preventDefault();
              this.openWikiLink.emit(linkText);
              return;
            }
          }
        }
      }
    }

    if (event.key === 'Enter' && event.ctrlKey) {
      event.preventDefault();
      this.statusCycleRequested.emit();
      return;
    }

    if (event.key === 'Enter' && event.shiftKey && !event.ctrlKey) {
      // Shift+Enter: Insert hard line break.
      // Flatmarkdown parser treats \n as a hard line break.
      event.preventDefault();
      const before = value.substring(0, selectionStart);
      const after = value.substring(selectionEnd);
      const newValue = before + '\n' + after;
      const newCursorPos = selectionStart + 1; // After the newline

      el.value = newValue;
      el.setSelectionRange(newCursorPos, newCursorPos);
      this.bodyChange.emit(newValue);
      this.autoResize(el);
      return;
    }

    if (event.key === 'Enter' && !event.shiftKey && !event.ctrlKey) {
      event.preventDefault();
      this.enterPressed.emit({ cursorPos: selectionStart });
      return;
    }

    if (event.key === 'Backspace' && selectionStart === 0 && selectionEnd === 0) {
      event.preventDefault();
      this.backspaceAtStart.emit();
      return;
    }

    if (event.key === 'Delete' && selectionStart === value.length && selectionEnd === value.length) {
      event.preventDefault();
      this.deleteAtEnd.emit();
      return;
    }

    if (event.ctrlKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      this.collapseToggled.emit();
      return;
    }

    if (event.shiftKey && event.altKey) {
      if (event.key === 'ArrowUp') {
        event.preventDefault();
        this.moveUp.emit();
        return;
      }
      if (event.key === 'ArrowDown') {
        event.preventDefault();
        this.moveDown.emit();
        return;
      }
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        this.outdentRequested.emit();
        return;
      }
      if (event.key === 'ArrowRight') {
        event.preventDefault();
        this.indentRequested.emit();
        return;
      }
    }

    if (event.shiftKey && !event.altKey && !event.ctrlKey) {
      if (event.key === 'ArrowUp' && this.isOnFirstLine(el)) {
        event.preventDefault();
        event.stopPropagation();
        this.shiftSelectUp.emit();
        return;
      }
      if (event.key === 'ArrowDown' && this.isOnLastLine(el)) {
        event.preventDefault();
        event.stopPropagation();
        this.shiftSelectDown.emit();
        return;
      }
    }

    if (event.key === 'ArrowUp' && this.isOnFirstLine(el)) {
      event.preventDefault();
      this.navigateUp.emit(selectionStart);
      return;
    }

    if (event.key === 'ArrowDown' && this.isOnLastLine(el)) {
      event.preventDefault();
      this.navigateDown.emit(selectionStart);
      return;
    }

    if (event.key === 'ArrowLeft' && selectionStart === 0) {
      event.preventDefault();
      // Emit large number to position cursor at end of previous item
      this.navigateUp.emit(Number.MAX_SAFE_INTEGER);
      return;
    }

    if (event.key === 'ArrowRight' && selectionStart === value.length) {
      event.preventDefault();
      // Position cursor at start of next item
      this.navigateDown.emit(0);
      return;
    }

    // Auto-close [[ with ]]
    if (event.key === '[' && selectionStart > 0 && value[selectionStart - 1] === '['
        && value.substring(selectionStart, selectionStart + 2) !== ']]') {
      event.preventDefault();
      const before = value.substring(0, selectionStart);
      const after = value.substring(selectionEnd);
      const newValue = before + '[]]' + after;
      const cursorPos = selectionStart + 1;
      el.value = newValue;
      el.setSelectionRange(cursorPos, cursorPos);
      this.bodyChange.emit(newValue);
      this.autoResize(el);
      return;
    }

    if (event.key === 'Tab') {
      event.preventDefault();
      if (event.shiftKey) {
        this.outdentRequested.emit();
      } else {
        this.indentRequested.emit();
      }
      return;
    }
  }

  /**
   * Handles paste events by preventing the default behavior and emitting
   * the clipboard data (plain text and HTML) along with the cursor position.
   * @param event - The clipboard paste event.
   */
  onPaste(event: ClipboardEvent): void {
    const el = event.target as HTMLTextAreaElement;
    const clipboardData = event.clipboardData;

    if (!clipboardData) return;

    // Check for image files in clipboard
    const imageFile = this.getImageFile(clipboardData);
    if (imageFile) {
      event.preventDefault();
      this.imagePasteDetected.emit({ file: imageFile, cursorPos: el.selectionStart });
      return;
    }

    // Get both plain text and HTML from clipboard
    // Normalize \r\n to \n — textarea normalizes internally but clipboard may contain \r\n on Windows
    const text = clipboardData.getData('text/plain').replace(/\r\n?/g, '\n');
    const html = clipboardData.getData('text/html');

    // Prevent default paste behavior - we'll handle it ourselves
    event.preventDefault();

    // Emit the paste event with clipboard data
    this.pasteDetected.emit({
      text,
      html,
      cursorPos: el.selectionStart
    });
  }

  private getImageFile(clipboardData: DataTransfer): File | null {
    for (let i = 0; i < clipboardData.items.length; i++) {
      const item = clipboardData.items[i];
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile();
        if (file) return file;
      }
    }
    return null;
  }

  /**
   * Handles mousedown on the textarea, initiating drag tracking
   * to detect when selection extends beyond the editor bounds.
   * @param event - The mouse event from the textarea.
   */
  onEditorMouseDown(event: MouseEvent): void {
    const el = event.target as HTMLTextAreaElement;
    this.textareaRect = el.getBoundingClientRect();
    this.isDragging = true;
    document.addEventListener('mousemove', this.mouseMoveHandler);
    document.addEventListener('mouseup', this.mouseUpHandler);
  }

  private onDocMouseMove(e: MouseEvent): void {
    if (!this.isDragging || !this.textareaRect) return;
    if (!this.dragSkipSelectionCheck) {
      const el = this.editorRef().nativeElement;
      // Only trigger if user has started selecting text
      if (el.selectionStart === el.selectionEnd) return;
    }

    const threshold = 10;
    if (e.clientY < this.textareaRect.top - threshold) {
      this.dragBeyondBounds.emit('above');
      this.onDocMouseUp();
    } else if (e.clientY > this.textareaRect.bottom + threshold) {
      this.dragBeyondBounds.emit('below');
      this.onDocMouseUp();
    }
  }

  private onDocMouseUp(): void {
    this.isDragging = false;
    this.dragSkipSelectionCheck = false;
    this.textareaRect = null;
    document.removeEventListener('mousemove', this.mouseMoveHandler);
    document.removeEventListener('mouseup', this.mouseUpHandler);
  }

  /** Cleans up document-level mouse event listeners on component destruction. */
  ngOnDestroy(): void {
    this.onDocMouseUp();
  }

  private checkWikiLinkContext(el: HTMLTextAreaElement): void {
    const { selectionStart, value } = el;
    // Look backward for [[ before cursor
    const before = value.substring(0, selectionStart);
    const openIdx = before.lastIndexOf('[[');
    if (openIdx === -1) {
      this.wikiLinkQuery.emit(null);
      return;
    }
    // Check there's no ]] between [[ and cursor (i.e., we're inside an open link)
    const between = before.substring(openIdx + 2);
    if (between.includes(']]')) {
      this.wikiLinkQuery.emit(null);
      return;
    }
    // Check there's a ]] after cursor
    const after = value.substring(selectionStart);
    if (!after.startsWith(']]') && !after.includes(']]')) {
      this.wikiLinkQuery.emit(null);
      return;
    }
    const query = between;
    const rect = this.getCaretRect(el, selectionStart);
    this.wikiLinkQuery.emit({ query, rect });
  }

  private getCaretRect(el: HTMLTextAreaElement, position: number): DOMRect {
    const div = document.createElement('div');
    const style = window.getComputedStyle(el);

    div.style.position = 'absolute';
    div.style.visibility = 'hidden';
    div.style.whiteSpace = 'pre-wrap';
    div.style.wordWrap = 'break-word';
    div.style.width = el.clientWidth + 'px';
    div.style.font = style.font;
    div.style.letterSpacing = style.letterSpacing;
    div.style.lineHeight = style.lineHeight;
    div.style.padding = style.padding;

    document.body.appendChild(div);

    const textBefore = document.createTextNode(el.value.substring(0, position));
    const marker = document.createElement('span');
    marker.textContent = '\u200B';
    div.appendChild(textBefore);
    div.appendChild(marker);

    const elRect = el.getBoundingClientRect();
    const markerLeft = marker.offsetLeft;
    const markerTop = marker.offsetTop;
    const lineHeight = parseFloat(style.lineHeight) || 20;

    document.body.removeChild(div);

    return new DOMRect(
      elRect.left + markerLeft - el.scrollLeft,
      elRect.top + markerTop - el.scrollTop,
      0,
      lineHeight
    );
  }

  /**
   * Replaces the current wiki link query text (between `[[` and `]]`) with the given replacement string.
   * @param replacement - The text to insert as the wiki link content.
   */
  insertWikiLinkText(replacement: string): void {
    const el = this.editorRef().nativeElement;
    const { selectionStart, value } = el;
    const before = value.substring(0, selectionStart);
    const openIdx = before.lastIndexOf('[[');
    if (openIdx === -1) return;
    const after = value.substring(selectionStart);
    const closeIdx = after.indexOf(']]');
    if (closeIdx === -1) return;
    const newValue = value.substring(0, openIdx + 2) + replacement + value.substring(selectionStart + closeIdx);
    const newCursorPos = openIdx + 2 + replacement.length;
    el.value = newValue;
    el.setSelectionRange(newCursorPos, newCursorPos);
    this.bodyChange.emit(newValue);
    this.autoResize(el);
  }

  private autoResize(el: HTMLTextAreaElement): void {
    el.style.height = 'auto';
    el.style.height = el.scrollHeight + 'px';
  }

  /**
   * Check if the cursor is on the first line of the textarea.
   */
  private isOnFirstLine(el: HTMLTextAreaElement): boolean {
    if (!el.value || el.selectionStart === 0) return true;
    const cursorY = this.measureYAtPosition(el, el.selectionStart);
    const firstLineY = this.measureYAtPosition(el, 0);
    return Math.abs(cursorY - firstLineY) < 5;
  }

  /**
   * Check if the cursor is on the last line of the textarea.
   */
  private isOnLastLine(el: HTMLTextAreaElement): boolean {
    if (!el.value || el.selectionStart === el.value.length) return true;
    const cursorY = this.measureYAtPosition(el, el.selectionStart);
    const endY = this.measureYAtPosition(el, el.value.length);
    return Math.abs(endY - cursorY) < 5;
  }

  /**
   * Measure the Y offset of a text position within a textarea using a mirror div.
   */
  private measureYAtPosition(el: HTMLTextAreaElement, position: number): number {
    const div = document.createElement('div');
    const style = window.getComputedStyle(el);

    div.style.position = 'absolute';
    div.style.visibility = 'hidden';
    div.style.whiteSpace = 'pre-wrap';
    div.style.wordWrap = 'break-word';
    div.style.width = el.clientWidth + 'px';
    div.style.font = style.font;
    div.style.letterSpacing = style.letterSpacing;
    div.style.lineHeight = style.lineHeight;

    document.body.appendChild(div);

    const textBefore = document.createTextNode(el.value.substring(0, position));
    const marker = document.createElement('span');
    marker.textContent = '|';
    div.appendChild(textBefore);
    div.appendChild(marker);

    const y = marker.offsetTop;
    document.body.removeChild(div);
    return y;
  }
}
