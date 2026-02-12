import { Component, input, output, computed, inject } from '@angular/core';
import { Block } from '../../core/models/block.model';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { BlockEditorComponent } from './block-editor';
import { BlockViewComponent } from './block-view';

@Component({
  selector: 'app-outline-block',
  imports: [BlockEditorComponent, BlockViewComponent],
  template: `
    <div
      class="outline-block"
      [style.padding-left.px]="indentPx()"
      [class.selected]="isSelected()"
    >
      <div class="block-controls">
        @if (hasChildren()) {
          <button
            class="collapse-toggle"
            (click)="toggleCollapse()"
            [class.collapsed]="isCollapsed()"
          >
            <span class="triangle">&#9654;</span>
          </button>
        } @else if (shouldShowBullet()) {
          <span class="bullet">&bull;</span>
        }
      </div>
      <div class="block-body">
        @if (isEditing()) {
          <app-block-editor
            [body]="block().body"
            [blockId]="block().id"
            [status]="block().status"
            (bodyChange)="bodyChanged.emit($event)"
            (blurred)="onEditFinished()"
            (enterPressed)="onEnterPressed($event)"
            (backspaceAtStart)="backspaceAtStart.emit()"
            (deleteAtEnd)="deleteAtEnd.emit()"
            (navigateUp)="navigateUp.emit($event)"
            (navigateDown)="navigateDown.emit($event)"
            (indentRequested)="indentRequested.emit()"
            (outdentRequested)="outdentRequested.emit()"
            (moveUp)="moveUp.emit()"
            (moveDown)="moveDown.emit()"
            (statusCycleRequested)="statusCycleRequested.emit()"
            (statusToggled)="taskToggled.emit()"
            (pasteDetected)="pasteDetected.emit($event)"
            (dragBeyondBounds)="dragBeyondBounds.emit($event)"
          />
        } @else {
          <app-block-view
            [html]="block().html"
            [status]="block().status"
            (clicked)="editRequested.emit($event)"
            (taskToggled)="taskToggled.emit()"
          />
        }
      </div>
    </div>
  `,
  styles: [`
    .outline-block {
      display: flex;
      align-items: flex-start;
      min-height: 28px;
    }
    .outline-block.selected {
      background: rgba(0, 120, 215, 0.1);
    }
    .block-controls {
      width: 24px;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      padding-top: 2px;
    }
    .collapse-toggle {
      background: none;
      border: none;
      cursor: pointer;
      padding: 0;
      font-size: 10px;
      color: #666;
      transition: transform 0.15s;
      line-height: 1;
      margin-top: 3px;
    }
    .collapse-toggle .triangle {
      display: inline-block;
      transition: transform 0.15s;
    }
    .collapse-toggle:not(.collapsed) .triangle {
      transform: rotate(90deg);
    }
    .bullet {
      color: #999;
      font-size: 14px;
    }
    .block-body {
      flex: 1;
      min-width: 0;
    }
  `]
})
export class OutlineBlockComponent {
  private readonly state = inject(OutlineStateService);

  readonly block = input.required<Block>();

  readonly bodyChanged = output<string>();
  readonly editRequested = output<number>();
  readonly editFinished = output<void>();
  readonly enterPressed = output<{ cursorPos: number }>();
  readonly backspaceAtStart = output<void>();
  readonly deleteAtEnd = output<void>();
  readonly navigateUp = output<number>();
  readonly navigateDown = output<number>();
  readonly indentRequested = output<void>();
  readonly outdentRequested = output<void>();
  readonly moveUp = output<void>();
  readonly moveDown = output<void>();
  readonly taskToggled = output<void>();
  readonly statusCycleRequested = output<void>();
  readonly collapseToggled = output<void>();
  readonly pasteDetected = output<{ text: string; html: string; cursorPos: number }>();
  readonly dragBeyondBounds = output<'above' | 'below'>();

  readonly indentPx = computed(() => this.block().level * 24);
  readonly hasChildren = computed(() => this.state.hasChildren(this.block().id));
  readonly isEditing = computed(() => this.state.editingBlockId() === this.block().id);
  readonly isSelected = computed(() => this.state.selectedBlockIds().has(this.block().id));
  readonly isCollapsed = computed(() => this.block().collapsed);
  readonly shouldShowBullet = computed(() => {
    // Don't show bullet if block has children (collapse toggle is shown instead)
    if (this.hasChildren()) return false;
    // In view mode (not editing), hide bullet for empty lines and lines starting with #
    if (!this.isEditing()) {
      const body = this.block().body.trim();
      if (body === '' || body.startsWith('#')) {
        return false;
      }
    }
    return true;
  });

  toggleCollapse(): void {
    this.collapseToggled.emit();
  }

  onEditFinished(): void {
    if (!this.state.suppressDuplicateAnimation) {
      this.createDuplicateTextarea();
    }
    this.editFinished.emit();
  }

  onEnterPressed(event: { cursorPos: number }): void {
    this.createDuplicateTextarea();
    this.enterPressed.emit(event);
  }

  private createDuplicateTextarea(): void {
    // Skip for blocks whose rendered style differs significantly from the textarea
    const trimmed = this.block().body.trimStart();
    if (trimmed.startsWith('#') || trimmed.startsWith('```') || trimmed.startsWith('>')) return;

    const textarea = document.getElementById('block-editor-' + this.block().id) as HTMLTextAreaElement;
    if (!textarea) return;

    // Create textarea duplicate with identical styling
    const duplicate = document.createElement('textarea');
    duplicate.value = textarea.value;
    duplicate.readOnly = true;
    duplicate.tabIndex = -1;
    duplicate.style.pointerEvents = 'none';

    // Copy all relevant styles from the original textarea
    const computedStyle = window.getComputedStyle(textarea);
    duplicate.style.font = computedStyle.font;
    duplicate.style.fontSize = computedStyle.fontSize;
    duplicate.style.fontFamily = computedStyle.fontFamily;
    duplicate.style.lineHeight = computedStyle.lineHeight;
    duplicate.style.padding = computedStyle.padding;
    duplicate.style.border = 'none';
    duplicate.style.outline = 'none';
    duplicate.style.resize = 'none';
    duplicate.style.background = 'transparent';
    duplicate.style.overflow = 'hidden';
    duplicate.style.color = computedStyle.color;
    duplicate.style.letterSpacing = computedStyle.letterSpacing;
    duplicate.style.wordSpacing = computedStyle.wordSpacing;

    // Get exact position
    const rect = textarea.getBoundingClientRect();
    duplicate.style.position = 'absolute';
    duplicate.style.left = (rect.left + window.scrollX) + 'px';
    duplicate.style.top = (rect.top + window.scrollY) + 'px';
    duplicate.style.width = rect.width + 'px';
    duplicate.style.height = rect.height + 'px';
    duplicate.style.zIndex = '0';
    duplicate.style.transition = 'opacity 0.7s ease-out';
    duplicate.style.opacity = '1';

    // Append to body
    document.body.appendChild(duplicate);

    // Force reflow so the browser registers opacity:1 before transitioning to 0
    duplicate.offsetHeight;

    // Trigger fade out
    duplicate.style.opacity = '0';

    // Remove after transition
    duplicate.addEventListener('transitionend', () => {
      duplicate.remove();
    });
  }
}
