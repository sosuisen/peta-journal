import { Component, input, output, ElementRef, viewChild, AfterViewInit, inject } from '@angular/core';
import { OutlineStateService } from '../../core/services/outline-state.service';

@Component({
  selector: 'app-block-editor',
  template: `
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
    :host { display: block; flex: 1; min-width: 0; }
    textarea {
      width: 100%;
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
  `]
})
export class BlockEditorComponent implements AfterViewInit {
  private readonly state = inject(OutlineStateService);
  readonly body = input.required<string>();
  readonly blockId = input.required<string>();

  readonly bodyChange = output<string>();
  readonly blurred = output<void>();
  readonly enterPressed = output<{ cursorPos: number }>();
  readonly backspaceAtStart = output<void>();
  readonly deleteAtEnd = output<void>();
  readonly navigateUp = output<void>();
  readonly navigateDown = output<void>();
  readonly indentRequested = output<void>();
  readonly outdentRequested = output<void>();
  readonly moveUp = output<void>();
  readonly moveDown = output<void>();

  private readonly editorRef = viewChild.required<ElementRef<HTMLTextAreaElement>>('editor');

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

  onInput(event: Event): void {
    const el = event.target as HTMLTextAreaElement;
    this.bodyChange.emit(el.value);
    this.autoResize(el);
  }

  onKeydown(event: KeyboardEvent): void {
    const el = event.target as HTMLTextAreaElement;
    const { selectionStart, selectionEnd, value } = el;

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

    if (event.key === 'ArrowUp' && selectionStart === 0) {
      event.preventDefault();
      this.navigateUp.emit();
      return;
    }

    if (event.key === 'ArrowDown' && selectionStart === value.length) {
      event.preventDefault();
      this.navigateDown.emit();
      return;
    }

    if (event.key === 'ArrowLeft' && selectionStart === 0) {
      event.preventDefault();
      this.navigateUp.emit();
      return;
    }

    if (event.key === 'ArrowRight' && selectionStart === value.length) {
      event.preventDefault();
      this.navigateDown.emit();
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
}
