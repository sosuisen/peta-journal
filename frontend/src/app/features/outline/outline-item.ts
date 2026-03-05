import { Component, input, output, computed, inject, viewChild } from '@angular/core';
import { Item } from '../../core/models/item.model';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ItemEditorComponent, WikiLinkContext } from './item-editor';
import { ItemViewComponent } from './item-view';

@Component({
  selector: 'app-outline-item',
  imports: [ItemEditorComponent, ItemViewComponent],
  template: `
    <div
      [id]="item().id"
      class="outline-item"
      [style.padding-left.px]="indentPx()"
      [class.selected]="isSelected()"
      [class.heading]="isHeading()"
    >
      <div class="item-controls">
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
      <div class="item-body">
        @if (isEditing()) {
          <app-item-editor
            #editor
            [body]="item().body"
            [itemId]="item().id"
            [types]="item().types"
            [properties]="item().properties"
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
            (imagePasteDetected)="imagePasteDetected.emit($event)"
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
          <app-item-view
            [ast]="item().ast"
            [types]="item().types"
            [properties]="item().properties"
            (clicked)="editRequested.emit($event)"
            (taskToggled)="taskToggled.emit()"
          />
        }
      </div>
      @if (taskTimeInfo(); as info) {
        <div class="task-timestamps" [title]="info.tooltip">{{ info.label }}</div>
      }
    </div>
  `,
  styles: [`
    .outline-item {
      display: flex;
      align-items: flex-start;
      min-height: 28px;
    }
    .outline-item.selected {
      background: rgba(0, 120, 215, 0.1);
    }
    .item-controls {
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
    .item-body {
      flex: 1;
      min-width: 0;
    }
    .task-timestamps {
      flex-shrink: 0;
      font-size: 12px;
      color: #999;
      padding-top: 4px;
      padding-left: 8px;
      white-space: nowrap;
    }
    .outline-item.heading .item-body {
      margin-left: -3px;
    }
  `]
})
export class OutlineItemComponent {
  private readonly state = inject(OutlineStateService);

  readonly item = input.required<Item>();

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
  readonly imagePasteDetected = output<{ file: File; cursorPos: number }>();
  readonly dragBeyondBounds = output<'above' | 'below'>();
  readonly wikiLinkQueryChanged = output<WikiLinkContext | null>();
  readonly wikiLinkSelected = output<string>();
  readonly openWikiLink = output<string>();
  readonly shiftSelectUp = output<void>();
  readonly shiftSelectDown = output<void>();
  readonly undoRequested = output<void>();
  readonly redoRequested = output<void>();
  readonly suggestionsVisible = input<boolean>(false);

  private readonly editorRef = viewChild<ItemEditorComponent>('editor');

  readonly indentPx = computed(() => (this.item().level - 1) * 24);
  readonly hasChildren = computed(() => this.state.hasChildren(this.item().id));
  readonly isEditing = computed(() => this.state.editingItemIds().has(this.item().id));
  readonly isSelected = computed(() => this.state.selectedItemIds().has(this.item().id));
  readonly isCollapsed = computed(() => this.item().collapsed);
  readonly isHeading = computed(() => this.item().body.trimStart().startsWith('#'));
  readonly taskTimeInfo = computed<{ label: string; tooltip: string } | null>(() => {
    const item = this.item();
    if (!item.types?.includes('task')) return null;
    const begin = item.properties['begin'] as string | undefined;
    const end = item.properties['end'] as string | undefined;
    if (!begin) return null;
    const fmt = (v: string) => {
      const d = new Date(v);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    };
    if (!end) return { label: `${fmt(begin)}-`, tooltip: `${fmt(begin)}-` };
    const diffMs = new Date(end).getTime() - new Date(begin).getTime();
    const totalSec = Math.floor(diffMs / 1000);
    const h = Math.floor(totalSec / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    let label = '';
    if (h > 0) label += `${h}h`;
    if (m > 0) label += `${m}m`;
    if (h === 0 && m === 0) label = `${s}s`;
    return { label, tooltip: `${fmt(begin)} - ${fmt(end)}` };
  });
  readonly shouldShowBullet = computed(() => {
    // Don't show bullet if item has children (collapse toggle is shown instead)
    if (this.hasChildren()) return false;
    // In view mode (not editing), hide bullet for empty lines and lines starting with #
    if (!this.isEditing()) {
      const body = this.item().body.trim();
      if (body === '' || body.startsWith('#')) {
        return false;
      }
    }
    return true;
  });

  /**
   * Inserts a wiki-link replacement text into the item editor at the current cursor position.
   * @param replacement - The wiki-link text to insert.
   */
  insertWikiLinkText(replacement: string): void {
    this.editorRef()?.insertWikiLinkText(replacement);
  }

  /** Emits a collapse toggle event for this item's children. */
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
