import { Component, input, output, computed, inject, viewChild, ElementRef } from '@angular/core';
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
      [attr.data-heading-level]="headingLevel() || null"
      [class.hide-bullet]="!shouldShowBullet()"
    >
      <div class="item-controls">
        <div class="slot-menu">
          @if (hasChildren()) {
            <button class="item-menu" (click)="onItemMenuClick($event)">⋯</button>
          }
        </div>
        <div class="slot-toggle">
          @if (hasChildren()) {
            <button
              class="collapse-toggle"
              (click)="toggleCollapse()"
              [class.collapsed]="isCollapsed()"
            >
              <svg class="triangle" viewBox="0 0 10 6" width="10" height="6">
                <polygon points="0,0 10,0 5,6" fill="currentColor" />
              </svg>
            </button>
          } @else {
            <button class="item-menu" (click)="onItemMenuClick($event)">⋯</button>
          }
        </div>
      </div>
      <div class="item-content" [style.zoom]="effectiveZoom()">
        <div class="slot-bullet">
          <span class="bullet">
            <svg viewBox="0 0 14 14" width="14" height="14">
              @if (isTransclusion()) {
                @if (isCollapsed()) {
                  <rect x="1" y="1" width="12" height="12" rx="1" fill="#ddd" stroke="none" />
                } @else {
                  <rect x="1" y="1" width="12" height="12" rx="1" fill="none" stroke="#ccc" stroke-width="1" />
                }
              } @else if (isCollapsed()) {
                <circle cx="7" cy="7" r="6.5" fill="#ddd" stroke="none" />
              }
              <circle cx="7" cy="7" r="2.5" fill="#888" stroke="none" />
            </svg>
          </span>
        </div>
        <div class="item-body">
        @if (isBrokenTransclusion()) {
          <div class="broken-transclusion">source not found: {{ item().body }}</div>
        } @else if (isEditing()) {
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
    </div>
    @if (showItemMenu) {
      <div class="item-popup-menu" [style.left.px]="menuX" [style.top.px]="menuY">
        <button (click)="onMenuCopy()">Copy</button>
        <button (click)="onMenuCut()">Cut</button>
        <button (click)="onMenuCopyReference()">Copy Reference</button>
        <button (click)="onMenuPasteAsTransclusion()">Paste as Transclusion</button>
        @if (isTransclusion()) {
          <button (click)="onMenuRemoveTransclusion()">Remove Transclusion</button>
        }
        <button (click)="onMenuSendToArchive()">Send to Archive</button>
        @if (hasZoom()) {
          <button (click)="onMenuResetZoom()">Reset Zoom</button>
        }
      </div>
    }
  `,
  styles: [`
    .outline-item {
      display: flex;
      align-items: flex-start;
      min-height: 28px;
    }
    .item-content {
      display: flex;
      align-items: flex-start;
      flex: 1;
      min-width: 0;
      transform-origin: left top;
    }
    .outline-item.selected {
      background: rgba(0, 120, 215, 0.1);
    }
    .item-controls {
      flex-shrink: 0;
      display: flex;
      align-items: center;
      padding-top: 4px;
    }
    .slot-menu {
      width: 18px;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .slot-toggle {
      width: 14px;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .slot-bullet {
      width: 16px;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      margin-right: 4px;
      padding-top: 5px;
    }
    .item-menu {
      background: none;
      border: none;
      cursor: pointer;
      padding: 0 1px;
      font-size: 16px;
      color: #999;
      line-height: 1;
      opacity: 0;
      transition: opacity 0.15s;
      letter-spacing: -1px;
    }
    .outline-item:hover .item-menu {
      opacity: 1;
    }
    .outline-item:hover .item-menu {
      color: #333;
    }
    .collapse-toggle {
      background: none;
      border: none;
      cursor: pointer;
      padding: 0 1px;
      font-size: 10px;
      color: #ddd;
      opacity: 0;
      transition: color 0.15s, opacity 0.15s;
      line-height: 1;
    }
    .outline-item:hover .collapse-toggle {
      opacity: 1;
      color: #333;
    }
    .collapse-toggle .triangle {
      display: block;
      transition: transform 0.15s;
    }
    .collapse-toggle.collapsed .triangle {
      transform: rotate(-90deg);
    }
    .bullet {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 16px;
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
    .outline-item.hide-bullet .bullet {
      opacity: 0;
      transition: opacity 0.15s;
    }
    .outline-item.hide-bullet:hover .bullet {
      opacity: 1;
    }
    .outline-item[data-heading-level="1"] .item-controls { padding-top: 12px; }
    .outline-item[data-heading-level="2"] .item-controls { padding-top: 10px; }
    .outline-item[data-heading-level="3"] .item-controls { padding-top: 8px; }
    .outline-item[data-heading-level="4"] .item-controls { padding-top: 4px; }
    .outline-item[data-heading-level="5"] .item-controls { padding-top: 3px; }
    .outline-item[data-heading-level="1"] .slot-bullet { padding-top: 13px; }
    .outline-item[data-heading-level="2"] .slot-bullet { padding-top: 11px; }
    .outline-item[data-heading-level="3"] .slot-bullet { padding-top: 9px; }
    .outline-item[data-heading-level="4"] .slot-bullet { padding-top: 5px; }
    .outline-item[data-heading-level="5"] .slot-bullet { padding-top: 4px; }
    .outline-item.heading .item-body {
      margin-left: -3px;
    }
    .broken-transclusion {
      color: #c00;
      font-style: italic;
      padding: 4px 0;
      font-size: 13px;
    }
    :host {
      position: relative;
    }
    .item-popup-menu {
      position: absolute;
      background: white;
      border: 1px solid #ddd;
      border-radius: 4px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.15);
      z-index: 1000;
      display: flex;
      flex-direction: column;
      min-width: 100px;
    }
    .item-popup-menu button {
      background: none;
      border: none;
      padding: 6px 12px;
      text-align: left;
      cursor: pointer;
      font-size: 13px;
    }
    .item-popup-menu button:hover {
      background: #f0f0f0;
    }
  `]
})
export class OutlineItemComponent {
  readonly state = inject(OutlineStateService);
  private readonly elRef = inject(ElementRef);

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
  readonly copyReference = output<void>();
  readonly pasteAsTransclusion = output<void>();
  readonly removeTransclusion = output<void>();
  readonly sendToArchive = output<void>();
  readonly resetZoom = output<void>();
  readonly suggestionsVisible = input<boolean>(false);
  readonly effectiveZoom = input<number>(1);

  private readonly editorRef = viewChild<ItemEditorComponent>('editor');

  readonly indentPx = computed(() => (this.item().level - 1) * 24);
  readonly hasChildren = computed(() => this.state.hasChildren(this.item().id));
  readonly isEditing = computed(() => this.state.editingItemIds().has(this.item().id));
  readonly isSelected = computed(() => this.state.selectedItemIds().has(this.item().id));
  readonly isCollapsed = computed(() => this.item().collapsed || this.state.isCollapsed(this.item().id));
  readonly isHeading = computed(() => this.item().body.trimStart().startsWith('#'));
  readonly headingLevel = computed(() => {
    const match = this.item().body.trimStart().match(/^(#{1,5})\s/);
    return match ? match[1].length : 0;
  });
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
  readonly isTransclusion = computed(() => !!this.item().transcludeItemId);
  readonly hasZoom = computed(() => {
    const z = parseFloat(this.item().properties['zoom'] as string);
    return !isNaN(z) && z !== 1;
  });
  readonly isBrokenTransclusion = computed(() => {
    const item = this.item();
    if (!item.transcludeItemId) return false;
    const trees = this.state.transcludedTrees();
    const tree = trees.get(item.transcludeItemId);
    return !tree || tree.length === 0;
  });
  readonly shouldShowBullet = computed(() => {
    // In view mode (not editing), hide bullet for empty lines and lines starting with #
    if (!this.isEditing()) {
      const body = this.item().body.trim();
      if (body === '') {
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

  showItemMenu = false;
  menuX = 0;
  menuY = 0;

  private closeMenuHandler: (() => void) | null = null;

  onItemMenuClick(event: MouseEvent): void {
    event.stopPropagation();
    const btn = event.currentTarget as HTMLElement;
    const btnRect = btn.getBoundingClientRect();
    const hostRect = this.elRef.nativeElement.getBoundingClientRect();
    this.menuX = btnRect.left - hostRect.left;
    this.menuY = btnRect.bottom - hostRect.top;
    this.showItemMenu = true;

    const closeMenu = (e?: Event) => {
      if (e && (e.target as HTMLElement).closest('.item-popup-menu')) return;
      this.showItemMenu = false;
      document.removeEventListener('mousedown', closeMenu, true);
      document.removeEventListener('keydown', onKey, true);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        closeMenu();
      }
    };
    setTimeout(() => {
      document.addEventListener('mousedown', closeMenu, true);
      document.addEventListener('keydown', onKey, true);
    });
  }

  onMenuCopy(): void {
    navigator.clipboard.writeText(this.item().body);
    this.showItemMenu = false;
  }

  onMenuCut(): void {
    navigator.clipboard.writeText(this.item().body);
    this.bodyChanged.emit('');
    this.showItemMenu = false;
  }

  onMenuCopyReference(): void {
    navigator.clipboard.writeText(this.item().id);
    this.showItemMenu = false;
  }

  onMenuPasteAsTransclusion(): void {
    this.pasteAsTransclusion.emit();
    this.showItemMenu = false;
  }

  onMenuRemoveTransclusion(): void {
    this.removeTransclusion.emit();
    this.showItemMenu = false;
  }

  onMenuSendToArchive(): void {
    this.sendToArchive.emit();
    this.showItemMenu = false;
  }

  onMenuResetZoom(): void {
    this.resetZoom.emit();
    this.showItemMenu = false;
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
