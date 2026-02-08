import { Component, input, output, ElementRef, viewChild, AfterViewInit, inject, effect } from '@angular/core';
import { OutlineStateService } from '../../core/services/outline-state.service';

@Component({
  selector: 'app-block-editor',
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
      [id]="'block-editor-' + blockId()"
      [value]="body()"
      (input)="onInput($event)"
      (keydown)="onKeydown($event)"
      (blur)="blurred.emit()"
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
export class BlockEditorComponent implements AfterViewInit {
  private readonly state = inject(OutlineStateService);
  readonly body = input.required<string>();
  readonly blockId = input.required<string>();
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

  private readonly editorRef = viewChild.required<ElementRef<HTMLTextAreaElement>>('editor');
  private readonly checkboxRef = viewChild<ElementRef<HTMLInputElement>>('checkbox');

  constructor() {
    effect(() => {
      const checkbox = this.checkboxRef()?.nativeElement;
      if (checkbox) {
        checkbox.indeterminate = this.status() === 'running';
      }
    });
  }

  ngAfterViewInit(): void {
    const el = this.editorRef().nativeElement;
    const pendingPos = this.state.consumePendingCursorPosition();
    if (pendingPos !== null) {
      const pos = Math.min(pendingPos, el.value.length);
      el.setSelectionRange(pos, pos);
    }
    el.focus();
    this.autoResize(el);
  }

  setCursorPosition(pos: number): void {
    const el = this.editorRef().nativeElement;
    el.setSelectionRange(pos, pos);
    el.focus();
  }

  setCursorToEnd(): void {
    const el = this.editorRef().nativeElement;
    const len = el.value.length;
    el.setSelectionRange(len, len);
    el.focus();
  }

  onCheckboxClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.statusToggled.emit();
  }

  onInput(event: Event): void {
    const el = event.target as HTMLTextAreaElement;
    this.bodyChange.emit(el.value);
    this.autoResize(el);
  }

  onKeydown(event: KeyboardEvent): void {
    const el = event.target as HTMLTextAreaElement;
    const { selectionStart, selectionEnd, value } = el;

    if (event.key === 'Enter' && event.ctrlKey) {
      event.preventDefault();
      this.statusCycleRequested.emit();
      return;
    }

    if (event.key === 'Enter' && event.shiftKey && !event.ctrlKey) {
      // Shift+Enter: Insert GFM hard line break (two spaces + newline)
      event.preventDefault();
      const before = value.substring(0, selectionStart);
      const after = value.substring(selectionEnd);
      const newValue = before + '  \n' + after;
      const newCursorPos = selectionStart + 3; // After the two spaces and newline

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
      // Emit large number to position cursor at end of previous block
      this.navigateUp.emit(Number.MAX_SAFE_INTEGER);
      return;
    }

    if (event.key === 'ArrowRight' && selectionStart === value.length) {
      event.preventDefault();
      // Position cursor at start of next block
      this.navigateDown.emit(0);
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

  private autoResize(el: HTMLTextAreaElement): void {
    el.style.height = 'auto';
    el.style.height = el.scrollHeight + 'px';
  }

  /**
   * Check if the cursor is on the first line of the textarea.
   * Uses a temporary element to measure line positions accounting for text wrapping.
   */
  private isOnFirstLine(el: HTMLTextAreaElement): boolean {
    const start = el.selectionStart;
    const value = el.value;

    // If empty or at position 0, definitely on first line
    if (!value || start === 0) return true;

    // Create a temporary div to measure text rendering
    const div = document.createElement('div');
    const style = window.getComputedStyle(el);

    // Copy relevant styles to match textarea rendering
    div.style.position = 'absolute';
    div.style.visibility = 'hidden';
    div.style.whiteSpace = 'pre-wrap';
    div.style.wordWrap = 'break-word';
    div.style.width = el.clientWidth + 'px';
    div.style.font = style.font;
    div.style.letterSpacing = style.letterSpacing;
    div.style.lineHeight = style.lineHeight;

    document.body.appendChild(div);

    // Create a marker span at cursor position
    const textBefore = document.createTextNode(value.substring(0, start));
    const marker = document.createElement('span');
    marker.textContent = '|';

    div.appendChild(textBefore);
    div.appendChild(marker);

    // Get Y position of cursor
    const cursorY = marker.offsetTop;

    // Get Y position of first character
    div.textContent = '';
    const firstChar = document.createElement('span');
    firstChar.textContent = value.charAt(0) || '|';
    div.appendChild(firstChar);
    const firstLineY = firstChar.offsetTop;

    document.body.removeChild(div);

    // Compare Y positions (within small tolerance for rounding)
    return Math.abs(cursorY - firstLineY) < 5;
  }

  /**
   * Check if the cursor is on the last line of the textarea.
   * Uses a temporary element to measure line positions accounting for text wrapping.
   */
  private isOnLastLine(el: HTMLTextAreaElement): boolean {
    const start = el.selectionStart;
    const value = el.value;

    // If empty or at end, definitely on last line
    if (!value || start === value.length) return true;

    // Create a temporary div to measure text rendering
    const div = document.createElement('div');
    const style = window.getComputedStyle(el);

    // Copy relevant styles to match textarea rendering
    div.style.position = 'absolute';
    div.style.visibility = 'hidden';
    div.style.whiteSpace = 'pre-wrap';
    div.style.wordWrap = 'break-word';
    div.style.width = el.clientWidth + 'px';
    div.style.font = style.font;
    div.style.letterSpacing = style.letterSpacing;
    div.style.lineHeight = style.lineHeight;

    document.body.appendChild(div);

    // Create a marker span at cursor position
    const textBefore = document.createTextNode(value.substring(0, start));
    const marker = document.createElement('span');
    marker.textContent = '|';

    div.appendChild(textBefore);
    div.appendChild(marker);

    // Get Y position of cursor
    const cursorY = marker.offsetTop;

    // Get Y position of last character
    div.textContent = value;
    const lastChar = document.createElement('span');
    lastChar.textContent = '|';
    div.appendChild(lastChar);
    const lastLineY = lastChar.offsetTop;

    document.body.removeChild(div);

    // Compare Y positions (within small tolerance for rounding)
    return Math.abs(lastLineY - cursorY) < 5;
  }
}
