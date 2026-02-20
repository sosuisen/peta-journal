import { Component, input, output, computed, inject, viewChild } from '@angular/core';
import { Block } from '../../core/models/block.model';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { BlockEditorComponent, WikiLinkContext } from './block-editor';
import { BlockViewComponent } from './block-view';

@Component({
  selector: 'app-outline-block',
  imports: [BlockEditorComponent, BlockViewComponent],
  template: `
    <div
      [id]="block().id"
      class="outline-block"
      [style.padding-left.px]="indentPx()"
      [class.selected]="isSelected()"
      [class.heading]="isHeading()"
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
            #editor
            [body]="block().body"
            [blockId]="block().id"
            [status]="block().status"
            [suggestionsVisible]="suggestionsVisible()"
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
            (wikiLinkQuery)="wikiLinkQueryChanged.emit($event)"
            (wikiLinkSelect)="wikiLinkSelected.emit($event)"
            (openWikiLink)="openWikiLink.emit($event)"
            (shiftSelectUp)="shiftSelectUp.emit()"
            (shiftSelectDown)="shiftSelectDown.emit()"
            (collapseToggled)="collapseToggled.emit()"
            (undoRequested)="undoRequested.emit()"
            (redoRequested)="redoRequested.emit()"
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
    .outline-block.heading .block-body {
      margin-left: -3px;
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
  readonly wikiLinkQueryChanged = output<WikiLinkContext | null>();
  readonly wikiLinkSelected = output<string>();
  readonly openWikiLink = output<string>();
  readonly shiftSelectUp = output<void>();
  readonly shiftSelectDown = output<void>();
  readonly undoRequested = output<void>();
  readonly redoRequested = output<void>();
  readonly suggestionsVisible = input<boolean>(false);

  private readonly editorRef = viewChild<BlockEditorComponent>('editor');

  readonly indentPx = computed(() => this.block().level * 24);
  readonly hasChildren = computed(() => this.state.hasChildren(this.block().id));
  readonly isEditing = computed(() => this.state.editingBlockId() === this.block().id);
  readonly isSelected = computed(() => this.state.selectedBlockIds().has(this.block().id));
  readonly isCollapsed = computed(() => this.block().collapsed);
  readonly isHeading = computed(() => this.block().body.trimStart().startsWith('#'));
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

  /**
   * Inserts a wiki-link replacement text into the block editor at the current cursor position.
   * @param replacement - The wiki-link text to insert.
   */
  insertWikiLinkText(replacement: string): void {
    this.editorRef()?.insertWikiLinkText(replacement);
  }

  /** Emits a collapse toggle event for this block's children. */
  toggleCollapse(): void {
    this.collapseToggled.emit();
  }

  onEditFinished(): void {
    this.editFinished.emit();
  }

  onEnterPressed(event: { cursorPos: number }): void {
    this.enterPressed.emit(event);
  }
}
