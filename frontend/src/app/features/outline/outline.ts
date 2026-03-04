import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild, viewChildren, signal, OnDestroy } from '@angular/core';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { OutlineItemComponent } from './outline-item';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { Item } from '../../core/models/item.model';
import { ClipboardService } from '../../core/services/clipboard.service';
import { UndoService, UndoEntry } from '../../core/services/undo.service';
import { formatJournalDate } from '../../core/utils/date.utils';
import { NavigationService } from '../../core/services/navigation.service';
import { WikiLinkContext, ensureCodeFenceClosed, hasUnclosedCodeFence } from './item-editor';
import { findMoveDownTarget, findMoveUpTarget } from './move-utils';
import { renderToHtml } from 'flatmarkdown-ast2html';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, OutlineItemComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport [itemSize]="40" [minBufferPx]="500" [maxBufferPx]="600" class="outline-viewport" (scrolledIndexChange)="onScrolledIndexChange($event)">
      <div
        *cdkVirtualFor="let item of state.visibleItems(); let i = index; trackBy: trackById"
        class="outline-item"
        (mouseenter)="onItemMouseEnter(item)"
      >
        @if (isFirstItemOfDate(i) && item.name !== currentPageDate() && item.type === 'journal') {
          <div class="date-header">
            {{ formatDateHeader(item.name) }}
          </div>
        }
        <app-outline-item
          #outlineItem
          [item]="item"
          [suggestionsVisible]="wikiSuggestionsItemId() === item.id && wikiSuggestions().length > 0"
          (bodyChanged)="onBodyChanged(item, $event)"
          (editRequested)="onEditRequested(item.id, $event)"
          (editFinished)="onEditFinished(item)"
          (enterPressed)="onEnterPressed(item, $event)"
          (backspaceAtStart)="onBackspaceAtStart(item)"
          (deleteAtEnd)="onDeleteAtEnd(item)"
          (navigateUp)="onNavigateUp(item, $event)"
          (navigateDown)="onNavigateDown(item, $event)"
          (indentRequested)="onIndent(item)"
          (outdentRequested)="onOutdent(item)"
          (moveUp)="onMoveUp(item)"
          (moveDown)="onMoveDown(item)"
          (taskToggled)="onTaskStatusToggled(item)"
          (statusCycleRequested)="onStatusCycleRequested(item)"
          (collapseToggled)="onCollapseToggled(item)"
          (pasteDetected)="onPaste(item, $event)"
          (imagePasteDetected)="onImagePaste(item, $event)"
          (dragBeyondBounds)="onDragBeyondBounds(item, $event)"
          (wikiLinkQueryChanged)="onWikiLinkQuery(item, $event)"
          (wikiLinkSelected)="onWikiLinkKeyNav($event)"
          (openWikiLink)="onOpenWikiLink($event)"
          (shiftSelectUp)="onShiftSelectUp(item)"
          (shiftSelectDown)="onShiftSelectDown(item)"
          (undoRequested)="performUndo()"
          (redoRequested)="performRedo()"
        />
      </div>
    </cdk-virtual-scroll-viewport>
    @if (wikiSuggestions().length > 0) {
      <div
        class="wiki-suggestions"
        [style.left.px]="wikiSuggestionsPos().x"
        [style.top.px]="wikiSuggestionsPos().y"
      >
        @for (suggestion of wikiSuggestions(); track suggestion; let i = $index) {
          <div
            class="wiki-suggestion-item"
            [class.selected]="i === wikiSuggestionIndex()"
            (mousedown)="onWikiSuggestionSelected(suggestion, $event)"
            (mouseenter)="wikiSuggestionIndex.set(i)"
          >
            {{ suggestion }}
          </div>
        }
      </div>
    }
  `,
  styles: [`
    :host { display: block; height: 100%; width: 100%; }
    .outline-viewport {
      height: 100%;
      width: 100% !important;
    }
    .outline-viewport ::ng-deep .cdk-virtual-scroll-content-wrapper {
      width: 100%;
      user-select: none;
    }
    .outline-item {
      width: 100%;
      box-sizing: border-box;
    }
    @media (min-width: 1024px) {
      .outline-item {
        max-width: 900px;
      }
    }
    .date-header {
      margin: 0;
      padding: 16px 0 8px 0;
      font-size: 1.5em;
      font-weight: bold;
      border-top: 1px solid #ddd;
    }
    .wiki-suggestions {
      position: fixed;
      background: #fff;
      border: 1px solid #ddd;
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      max-height: 200px;
      overflow-y: auto;
      z-index: 1000;
      min-width: 200px;
      max-width: 400px;
      padding: 4px 0;
    }
    .wiki-suggestion-item {
      padding: 6px 12px;
      cursor: pointer;
      font-size: 13px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .wiki-suggestion-item.selected {
      background: #e8f0fe;
    }
    .wiki-suggestion-item:hover {
      background: #f0f0f0;
    }
  `]
})
export class OutlineComponent implements OnDestroy {
  readonly state = inject(OutlineStateService);
  private readonly api = inject(ApiService);
  private readonly autoSave = inject(AutoSaveService);
  private readonly injector = inject(Injector);
  private readonly clipboard = inject(ClipboardService);
  private readonly nav = inject(NavigationService);
  private readonly undo = inject(UndoService);
  readonly pageId = input.required<string>();
  readonly focusItemId = input<string | null | undefined>('');
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic item loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  readonly topItemChanged = output<string>();  // Emits name of top visible item
  readonly scrolledFromTop = output<boolean>();  // Emits true when scrolled away from top
  private readonly dirtyItemIds = new Set<string>();
  private switchingEditItem = false;
  private movingEditItem = false;
  private operatingEnterKey = false;  // Prevents onEditFinished from flushing
  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');
  private readonly outlineItems = viewChildren<OutlineItemComponent>('outlineItem');

  // Wiki link suggestions state
  readonly wikiSuggestions = signal<string[]>([]);
  readonly wikiSuggestionIndex = signal(0);
  readonly wikiSuggestionsItemId = signal<string | null>(null);
  readonly wikiSuggestionsPos = signal({ x: 0, y: 0 });
  private wikiSearchTimer: ReturnType<typeof setTimeout> | null = null;

  private selectionAnchorItemId: string | null = null;
  private selectionExtentItemId: string | null = null;
  private isSelectionDragging = false;
  private selectionKeydownHandler = (e: KeyboardEvent) => this.handleSelectionKeydown(e);
  private selectionMouseUpHandler = () => this.onSelectionMouseUp();
  private selectionClickHandler = (e: MouseEvent) => this.onSelectionClick(e);
  private selectionCopyHandler = (e: ClipboardEvent) => this.onSelectionCopy(e);
  private selectionCutHandler = (e: ClipboardEvent) => this.onSelectionCut(e);

  constructor() {
    effect(() => {
      if (this.disableAutoLoad()) return;  // Skip auto-loading if disabled
      const id = this.pageId();
      if (id) this.loadItems(id);
    });

    // Clear undo/redo stacks on page change
    effect(() => {
      this.pageId();
      this.undo.clear();
    });
  }

  /**
   * Handles the virtual scroll viewport emitting a new scrolled index.
   * Emits the top visible item's name and triggers loading more items when nearing the end.
   * @param index - The index of the first visible item in the virtual scroll viewport
   */
  onScrolledIndexChange(index: number): void {
    const items = this.state.visibleItems();
    const totalItems = items.length;

    this.scrolledFromTop.emit(index > 0);

    // Emit the top visible item's name
    if (index >= 0 && index < totalItems) {
      const topItem = items[index];
      this.topItemChanged.emit(topItem.name);
    }

    // Check if we need more buffer (100 items ahead)
    if (totalItems < index + 100 && totalItems > 0) {
      const lastItem = items[totalItems - 1];
      const filepath = lastItem.dirPath + lastItem.name;
      this.loadMore.emit({ filepath, currentIndex: index });
    }
  }

  /**
   * TrackBy function for the virtual scroll's cdkVirtualFor directive.
   * @param _index - The index of the item (unused)
   * @param item - The item to track
   * @returns The item's unique identifier
   */
  trackById(_index: number, item: Item): string {
    return item.id;
  }

  /**
   * Determines whether the item at the given index is the first item of a new date group.
   * Used in the template to render date header separators between journal entries.
   * @param index - The index of the item in the visible items array
   * @returns True if this item starts a new date group
   */
  isFirstItemOfDate(index: number): boolean {
    const items = this.state.visibleItems();
    if (index === 0) return true;
    const currentItem = items[index];
    const previousItem = items[index - 1];
    return currentItem.name !== previousItem.name;
  }

  /**
   * Formats a journal file name into a human-readable date string for display as a date header.
   * @param fileName - The journal file name to format
   * @returns A formatted date string
   */
  formatDateHeader(fileName: string): string {
    return formatJournalDate(fileName);
  }

  private async loadItems(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPageById(pageId);
      this.state.setItems(result.items);
      this.scrollToFocusItem();
    } catch (e) {
      console.error('Failed to load items for page:', pageId, e);
    }
  }

  private scrollToFocusItem(): void {
    const itemId = this.focusItemId();
    if (!itemId) return;
    const items = this.state.visibleItems();
    const index = items.findIndex(b => b.id === itemId);
    if (index < 0) return;
    const subtreeIds = this.state.getChildren(itemId).map(b => b.id);
    this.state.setSelectedItemIds(new Set([itemId, ...subtreeIds]));
    this.state.setItemSelectionMode(true);
    this.selectionAnchorItemId = itemId;
    this.selectionExtentItemId = itemId;
    document.addEventListener('keydown', this.selectionKeydownHandler);
    document.addEventListener('click', this.selectionClickHandler);
    document.addEventListener('copy', this.selectionCopyHandler);
    document.addEventListener('cut', this.selectionCutHandler);
    afterNextRender(() => {
      this.viewport()?.scrollToIndex(index);
    }, { injector: this.injector });
  }

  /**
   * Handles the user clicking on a item to start editing it.
   * Ignored when item selection mode is active.
   * @param itemId - The ID of the item to edit
   * @param cursorPosition - The cursor position to place the caret at
   */
  async onEditRequested(itemId: string, cursorPosition: number): Promise<void> {
    if (this.state.itemSelectionMode()) return;
    const currentId = this.state.focusedItemId();
    if (currentId && currentId !== itemId) {
      const current = this.state.getItemById(currentId);
      if (current) {
        this.switchingEditItem = true;
        await this.saveItemIfDirty(current);
        this.switchingEditItem = false;
      }
    }
    this.state.addEditingItem(itemId);
    this.state.setFocusedItem(itemId, cursorPosition);
    this.state.setPendingMouseDrag(true);
  }

  /**
   * Handles the user typing in a item editor, updating local state and scheduling an auto-save.
   * @param item - The item being edited
   * @param newBody - The updated body text
   */
  onBodyChanged(item: Item, newBody: string): void {
    const before = { ...item };
    this.state.updateItem(item.id, { body: newBody });
    this.dirtyItemIds.add(item.id);
    this.autoSave.scheduleItemUpdate(item.id, newBody);
    const after = { ...item, body: newBody };
    this.undo.begin('body');
    this.undo.pushUpdate(before, after);
    this.undo.commit();
  }

  /**
   * Handles the user leaving a item editor (blur). Flushes pending changes to the backend
   * and switches the item back to view mode with updated HTML.
   * @param item - The item that was being edited
   */
  async onEditFinished(item: Item): Promise<void> {
    this.closeWikiSuggestions();
    if (this.switchingEditItem) return;  // onEditRequested is handling save + switch

    if (this.movingEditItem) return;  // onMoveUp/onMoveDown is handling save + switch

    // If not in editing set, another handler already removed it
    if (!this.state.editingItemIds().has(item.id)) return;

    if (this.operatingEnterKey) return;

    if (!this.dirtyItemIds.has(item.id)) {
      const current = this.state.getItemById(item.id);
      if (current && hasUnclosedCodeFence(current.body)) {
        await this.saveItemBody(item.id, current.body);
      }
      this.state.removeEditingItem(item.id);
      return;
    }
    // Save dirty item
    this.dirtyItemIds.delete(item.id);
    await this.autoSave.flushItem(item.id);
    const current = this.state.getItemById(item.id);
    if (current) {
      await this.saveItemBody(item.id, current.body);
    }
    this.state.removeEditingItem(item.id);
  }

  /**
   * Handles the user pressing Enter in a item editor. Splits the item at the cursor position,
   * creating a new item with the text after the cursor. If the item is empty and indented,
   * outdents instead. If the cursor is at the beginning, inserts an empty item before.
   * @param item - The item where Enter was pressed
   * @param event - Contains the cursor position at the time of the keypress
   */
  async onEnterPressed(item: Item, event: { cursorPos: number }): Promise<void> {
    this.autoSave.cancelItem(item.id);
    this.dirtyItemIds.delete(item.id);

    const { cursorPos } = event;
    const beforeText = item.body.substring(0, cursorPos);
    const afterText = item.body.substring(cursorPos);

    if (item.body.length === 0 && item.level > 1) {
      // Check if there are same-level siblings below within the same parent
      const vItems = this.state.visibleItems();
      const vIdx = vItems.findIndex(b => b.id === item.id);
      let hasSiblingBelow = false;
      for (let i = vIdx + 1; i < vItems.length; i++) {
        if (vItems[i].level < item.level) break;
        if (vItems[i].level === item.level) { hasSiblingBelow = true; break; }
      }
      if (!hasSiblingBelow) {
        // No siblings below: outdent instead of creating new item
        await this.onOutdent(item);
        this.focusEditor(item.id);
        return;
      }
      // Has siblings below: fall through to create new item
    }

    this.operatingEnterKey = true;

    // Determine level for new item: if the original item has a direct child
    // (next item at level+1) immediately below and is not collapsed, inherit deeper level.
    const allItems = this.state.items();
    const itemIdx = allItems.findIndex(b => b.id === item.id);
    const nextItem = itemIdx >= 0 && itemIdx < allItems.length - 1 ? allItems[itemIdx + 1] : undefined;
    const newLevel = (!item.collapsed && nextItem && nextItem.level === item.level + 1)
      ? item.level + 1
      : item.level;

    // For collapsed items, insert after the last descendant (end of subtree)
    const descendants = item.collapsed ? this.state.getChildren(item.id) : [];
    const afterItemId = descendants.length > 0 ? descendants[descendants.length - 1].id : item.id;

    if (cursorPos === 0 && item.body.length > 0) {
      // Enter at beginning: create new empty item before current item
      const prevItemId = itemIdx > 0 ? allItems[itemIdx - 1].id : '';

      const [newItem] = await Promise.all([
        this.api.createItem(item.pageId, prevItemId, item.level, ''),
        this.saveItemBody(item.id, item.body),
      ]);
      this.state.addItem(newItem, prevItemId);

      this.undo.begin('enter', { itemId: item.id, position: cursorPos });
      this.undo.pushCreate(newItem);
      this.undo.commit({ itemId: newItem.id, position: 0 });

      this.state.addEditingItem(newItem.id);
      this.focusEditor(newItem.id);

      this.state.removeEditingItem(item.id);
    } else if (cursorPos === item.body.length) {
      // Enter at end: body unchanged — add optimistic item with client-generated ID
      const newId = crypto.randomUUID();
      const optimisticItem: Item = {
        id: newId,
        pageId: item.pageId,
        dirPath: item.dirPath,
        name: item.name,
        type: item.type,
        createdDate: Date.now(),
        modifiedDate: Date.now(),
        idx: '',
        level: newLevel,
        body: '',
        ast: '',
        collapsed: false,
        types: [],
        properties: {},
      };
      this.api.createItem(item.pageId, afterItemId, newLevel, '', newId).then(serverItem => {
        this.state.updateItem(newId, { idx: serverItem.idx });
      });
      this.state.addItem(optimisticItem, afterItemId);

      this.undo.begin('enter', { itemId: item.id, position: cursorPos });
      this.undo.pushUpdate({ ...item }, { ...item });
      this.undo.pushCreate(optimisticItem);
      this.undo.commit({ itemId: newId, position: 0 });

      this.dirtyItemIds.delete(item.id);
      this.autoSave.cancelItem(item.id);
      this.state.addEditingItem(newId);
      this.state.setFocusedItem(newId);
      this.focusEditor(newId);

      await this.saveItemBody(item.id, beforeText);
      this.state.removeEditingItem(item.id);

    } else {
      const originalItem = { ...item };

      const [updatedOriginal, newItem] = await Promise.all([
        this.saveItemBody(item.id, beforeText),
        this.api.createItem(item.pageId, afterItemId, newLevel, afterText),
      ]);
      this.state.addItem(newItem, afterItemId);

      this.undo.begin('enter', { itemId: item.id, position: cursorPos });
      this.undo.pushUpdate(originalItem, updatedOriginal);
      this.undo.pushCreate(newItem);
      this.undo.commit({ itemId: newItem.id, position: 0 });

      this.state.addEditingItem(newItem.id);
      this.state.setFocusedItem(newItem.id);
      this.focusEditor(newItem.id);

      this.state.removeEditingItem(item.id);
    }

    this.operatingEnterKey = false;
  }

  /**
   * Handles the user pressing Backspace at the start of a item.
   * Merges the current item's content into the previous item and deletes the current item.
   * @param item - The item where Backspace was pressed at position 0
   */
  async onBackspaceAtStart(item: Item): Promise<void> {
    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx <= 0) return;

    this.autoSave.cancelItem(item.id);
    this.dirtyItemIds.delete(item.id);

    const prevItem = items[idx - 1];
    const originalPrev = { ...prevItem };
    const originalItem = { ...item };
    const cursorPos = prevItem.body.length;
    const mergedBody = prevItem.body + item.body;
    this.state.updateItem(prevItem.id, { body: mergedBody });
    this.state.deleteItem(item.id);
    this.state.addEditingItem(prevItem.id);
    this.focusEditor(prevItem.id);

    const updatedPrev = await this.saveItemBody(prevItem.id, mergedBody);
    await this.api.deleteItem(item.id);

    this.undo.begin('backspace', { itemId: item.id, position: 0 });
    this.undo.pushUpdate(originalPrev, updatedPrev);
    this.undo.pushDelete(originalItem);
    this.undo.commit({ itemId: prevItem.id, position: cursorPos });
  }

  /**
   * Handles the user pressing Delete at the end of a item.
   * Merges the next item's content into the current item and deletes the next item.
   * @param item - The item where Delete was pressed at the end
   */
  async onDeleteAtEnd(item: Item): Promise<void> {
    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx >= items.length - 1) return;

    const nextItem = items[idx + 1];
    this.autoSave.cancelItem(nextItem.id);
    this.dirtyItemIds.delete(nextItem.id);

    const originalItem = { ...item };
    const originalNext = { ...nextItem };
    const cursorPos = item.body.length;
    const mergedBody = item.body + nextItem.body;
    this.state.updateItem(item.id, { body: mergedBody });
    this.state.deleteItem(nextItem.id);

    const updatedItem = await this.saveItemBody(item.id, mergedBody);
    await this.api.deleteItem(nextItem.id);

    this.undo.begin('delete', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate(originalItem, updatedItem);
    this.undo.pushDelete(originalNext);
    this.undo.commit({ itemId: item.id, position: cursorPos });
  }

  /**
   * Handles the user pressing the Up arrow key to navigate to the previous item.
   * Saves any pending changes before moving focus.
   * @param item - The item where the Up arrow was pressed
   * @param cursorPos - The cursor position to restore in the target item
   */
  async onNavigateUp(item: Item, cursorPos: number): Promise<void> {
    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx > 0) {
      // Save and update HTML before navigating
      await this.saveItemIfDirty(item);
      this.state.addEditingItem(items[idx - 1].id);
      this.state.setFocusedItem(items[idx - 1].id, cursorPos);
      this.state.removeEditingItem(item.id);
    }
  }

  /**
   * Handles the user pressing the Down arrow key to navigate to the next item.
   * Saves any pending changes before moving focus.
   * @param item - The item where the Down arrow was pressed
   * @param cursorPos - The cursor position to restore in the target item
   */
  async onNavigateDown(item: Item, cursorPos: number): Promise<void> {
    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx < items.length - 1) {
      // Save and update HTML before navigating
      await this.saveItemIfDirty(item);
      this.state.addEditingItem(items[idx + 1].id);
      this.state.setFocusedItem(items[idx + 1].id, cursorPos);
      this.state.removeEditingItem(item.id);
    }
  }

  private async saveItemIfDirty(item: Item): Promise<void> {
    if (!this.dirtyItemIds.has(item.id)) return;

    this.dirtyItemIds.delete(item.id);
    await this.autoSave.flushItem(item.id);
    const current = this.state.getItemById(item.id);
    if (current) {
      await this.saveItemBody(item.id, current.body);
      this.state.removeEditingItem(item.id);
    }
  }

  /**
   * Check if a item at the given level can be placed after prevItem.
   * Constraint: the previous item's level must be the same as, or exactly 1 level above (parent).
   * i.e. prevLevel === itemLevel (sibling) or prevLevel === itemLevel - 1 (parent)
   * If no previous item, the item must be at level 0 (root).
   */
  private canPlace(itemLevel: number, prevItem: Item | undefined): boolean {
    if (!prevItem) return itemLevel === 1;
    return prevItem.level === itemLevel || prevItem.level === itemLevel - 1;
  }

  /**
   * Handles the user pressing Tab to indent a item and its children by one level.
   * Validates placement constraints before allowing the indent.
   * @param item - The item to indent
   */
  async onIndent(item: Item): Promise<void> {
    const items = this.state.items();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx <= 0) return;
    const prevItem = items[idx - 1];
    const newLevel = item.level + 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevItem)) {
      // Exception: indenting by 1 level is allowed if there is a sibling at the same level
      // within the same parent subtree (stop scanning at shallower levels)
      let hasSameLevelBefore = false;
      for (let i = idx - 1; i >= 0; i--) {
        if (items[i].level < item.level) break; // exited current subtree
        if (items[i].level === item.level) {
          hasSameLevelBefore = true;
          break;
        }
      }
      if (!hasSameLevelBefore) return;
    }

    const group = this.state.getItemWithChildren(item.id);
    this.undo.begin('indent');
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + 1 });
      this.state.updateItem(b.id, { level: b.level + 1 });
    }
    this.undo.commit();
    const results = await Promise.all(group.map(b => this.api.updateItem(b.id, { level: b.level + 1 })));
    for (const r of results) {
      this.state.updateItem(r.id, { idx: r.idx, level: r.level });
    }
  }

  /**
   * Handles the user pressing Shift+Tab to outdent a item and its children by one level.
   * May reposition the item after its parent's last child if normal placement is not possible.
   * @param item - The item to outdent
   */
  async onOutdent(item: Item): Promise<void> {
    if (item.level <= 1) return;
    const items = this.state.items();
    const idx = items.findIndex(b => b.id === item.id);
    const prevItem = idx > 0 ? items[idx - 1] : undefined;
    const newLevel = item.level - 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevItem)) {
      // Exception: outdenting by 1 level is allowed by placing after the last child of the current parent
      let currentParentIdx = -1;
      for (let i = idx - 1; i >= 0; i--) {
        if (items[i].level === item.level - 1) {
          currentParentIdx = i;
          break;
        }
      }

      if (currentParentIdx >= 0) {
        const currentParent = items[currentParentIdx];
        const allChildren = this.state.getChildren(currentParent.id);
        const group = this.state.getItemWithChildren(item.id);
        const movingIds = new Set(group.map(b => b.id));

        let lastChildId = currentParent.id;
        for (const child of allChildren) {
          if (!movingIds.has(child.id)) {
            lastChildId = child.id;
          }
        }

        const groupIds = group.map(b => b.id);
        const beforeSnapshots = group.map(b => ({ ...b }));

        // Update levels
        for (const b of group) {
          this.state.updateItem(b.id, { level: b.level - 1 });
        }
        const levelResults = await Promise.all(group.map(b => this.api.updateItem(b.id, { level: b.level - 1 })));
        for (const r of levelResults) {
          this.state.updateItem(r.id, { idx: r.idx, level: r.level });
        }

        // Move after the last child (if it's not the parent itself)
        if (lastChildId !== currentParent.id) {
          const moved = await this.api.moveItems(groupIds, item.pageId, lastChildId);
          this.state.replaceItems(groupIds, moved, lastChildId);

          this.undo.begin('outdent');
          for (let i = 0; i < beforeSnapshots.length; i++) {
            this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
          }
          this.undo.commit();
        } else {
          this.undo.begin('outdent');
          for (const b of beforeSnapshots) {
            this.undo.pushUpdate(b, { ...b, level: b.level - 1 });
          }
          this.undo.commit();
        }
        return;
      }

      return; // Cannot place
    }

    // Normal outdent
    const group = this.state.getItemWithChildren(item.id);
    this.undo.begin('outdent');
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level - 1 });
      this.state.updateItem(b.id, { level: b.level - 1 });
    }
    this.undo.commit();
    const outdentResults = await Promise.all(group.map(b => this.api.updateItem(b.id, { level: b.level - 1 })));
    for (const r of outdentResults) {
      this.state.updateItem(r.id, { idx: r.idx, level: r.level });
    }
  }

  /**
   * Handles the user pressing Alt+Shift+Up to move a item and its children up
   * past the previous sibling tree.
   * @param item - The item to move up
   */
  async onMoveUp(item: Item): Promise<void> {
    const target = findMoveUpTarget(this.state.items(), item.id);
    if (!target) return;

    const group = this.state.getItemWithChildren(item.id);
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    const moved = await this.api.moveItems(groupIds, item.pageId, target.afterItemId);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    this.movingEditItem = true;
    this.state.replaceItems(groupIds, moved, target.afterItemId);
    this.focusEditor(item.id, () => {
      this.movingEditItem = false;
    });
  }

  /**
   * Handles the user pressing Alt+Shift+Down to move a item and its children down
   * past the next sibling tree.
   * @param item - The item to move down
   */
  async onMoveDown(item: Item): Promise<void> {
    const target = findMoveDownTarget(this.state.items(), item.id);
    if (!target) return;

    const children = this.state.getChildren(item.id);
    const group = [item, ...children];
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    const moved = await this.api.moveItems(groupIds, item.pageId, target.afterItemId);
    this.state.replaceItems(groupIds, moved, target.afterItemId);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    this.focusEditor(item.id);
  }

  /**
   * Handles the user typing inside a `[[wiki link]]` in the item editor.
   * Positions the suggestion popup and performs a debounced search for matching page names.
   * @param item - The item being edited
   * @param ctx - The wiki link context with query text and caret position, or null to close suggestions
   */
  onWikiLinkQuery(item: Item, ctx: WikiLinkContext | null): void {
    if (!ctx) {
      this.closeWikiSuggestions();
      return;
    }
    this.wikiSuggestionsItemId.set(item.id);
    this.wikiSuggestionsPos.set({ x: ctx.rect.left, y: ctx.rect.bottom + 4 });

    if (!ctx.query) {
      this.wikiSuggestions.set([]);
      return;
    }

    // Debounce search
    if (this.wikiSearchTimer) clearTimeout(this.wikiSearchTimer);
    this.wikiSearchTimer = setTimeout(async () => {
      try {
        const result = await this.api.searchPageNames(ctx.query, 10);
        const titles = result.results.map(r => r.pageTitle);
        this.wikiSuggestions.set(titles);
        this.wikiSuggestionIndex.set(0);
      } catch (e) {
        this.wikiSuggestions.set([]);
      }
    }, 150);
  }

  /**
   * Handles the user pressing arrow keys or Enter while the wiki link suggestion popup is open.
   * Navigates the suggestion list or selects the highlighted suggestion.
   * @param key - The key pressed ('ArrowUp', 'ArrowDown', or 'Enter')
   */
  onWikiLinkKeyNav(key: string): void {
    const suggestions = this.wikiSuggestions();
    if (suggestions.length === 0) return;
    const idx = this.wikiSuggestionIndex();

    if (key === 'ArrowDown') {
      this.wikiSuggestionIndex.set((idx + 1) % suggestions.length);
    } else if (key === 'ArrowUp') {
      this.wikiSuggestionIndex.set((idx - 1 + suggestions.length) % suggestions.length);
    } else if (key === 'Enter') {
      this.onWikiSuggestionSelected(suggestions[idx]);
    }
  }

  /**
   * Handles the user clicking on a wiki link to navigate to the linked page.
   * @param linkText - The text content of the wiki link (page title)
   */
  onOpenWikiLink(linkText: string): void {
    this.nav.navigateToPage(linkText);
  }

  /**
   * Handles the user pressing Shift+Up to begin or extend a multi-item selection upward.
   * @param item - The item where Shift+Up was pressed
   */
  onShiftSelectUp(item: Item): void {
    this.startShiftSelection(item);
  }

  /**
   * Handles the user pressing Shift+Down to begin or extend a multi-item selection downward.
   * @param item - The item where Shift+Down was pressed
   */
  onShiftSelectDown(item: Item): void {
    this.startShiftSelection(item);
  }

  private startShiftSelection(item: Item): void {
    this.state.removeAllEditingItems();
    this.state.setItemSelectionMode(true);
    this.selectionAnchorItemId = item.id;
    this.selectionExtentItemId = item.id;
    // Initial selection: only the current item, no subtree
    this.state.setSelectedItemIds(new Set([item.id]));

    document.addEventListener('keydown', this.selectionKeydownHandler);
    document.addEventListener('click', this.selectionClickHandler);
    document.addEventListener('copy', this.selectionCopyHandler);
    document.addEventListener('cut', this.selectionCutHandler);
  }

  private shiftExtendUp(): void {
    const items = this.state.visibleItems();
    let extentIdx = items.findIndex(b => b.id === this.selectionExtentItemId);
    const anchorIdx = items.findIndex(b => b.id === this.selectionAnchorItemId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx <= 0) return;

    // When expanding upward, skip past items already in selection
    if (extentIdx <= anchorIdx) {
      const selected = this.state.selectedItemIds();
      while (extentIdx - 1 >= 0 && selected.has(items[extentIdx - 1].id)) {
        extentIdx--;
      }
      if (extentIdx <= 0) return;
    }

    this.selectionExtentItemId = items[extentIdx - 1].id;
    this.updateShiftSelection();
  }

  private shiftExtendDown(): void {
    const items = this.state.visibleItems();
    let extentIdx = items.findIndex(b => b.id === this.selectionExtentItemId);
    const anchorIdx = items.findIndex(b => b.id === this.selectionAnchorItemId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx >= items.length - 1) return;

    // When expanding downward, skip past items already in selection
    if (extentIdx >= anchorIdx) {
      const selected = this.state.selectedItemIds();
      while (extentIdx + 1 < items.length && selected.has(items[extentIdx + 1].id)) {
        extentIdx++;
      }
      if (extentIdx >= items.length - 1) return;
    }

    this.selectionExtentItemId = items[extentIdx + 1].id;
    this.updateShiftSelection();
  }

  private updateShiftSelection(): void {
    const items = this.state.visibleItems();
    const anchorIdx = items.findIndex(b => b.id === this.selectionAnchorItemId);
    const extentIdx = items.findIndex(b => b.id === this.selectionExtentItemId);
    if (anchorIdx < 0 || extentIdx < 0) return;

    const start = Math.min(anchorIdx, extentIdx);
    const end = Math.max(anchorIdx, extentIdx);
    const selectedSet = new Set<string>();
    for (let i = start; i <= end; i++) {
      const group = this.state.getItemWithChildren(items[i].id);
      group.forEach(b => selectedSet.add(b.id));
    }
    this.state.setSelectedItemIds(selectedSet);
  }

  /**
   * Handles the user selecting a wiki link suggestion by clicking on it or pressing Enter.
   * Inserts the selected page title into the item editor and closes the suggestions popup.
   * @param title - The page title selected from the suggestions
   * @param event - The mouse event if selected by click (used to prevent default behavior)
   */
  onWikiSuggestionSelected(title: string, event?: MouseEvent): void {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    const itemId = this.wikiSuggestionsItemId();
    if (!itemId) return;
    const itemComp = this.outlineItems()?.find(
      ob => ob.item().id === itemId
    );
    itemComp?.insertWikiLinkText(title);
    this.closeWikiSuggestions();
  }

  private closeWikiSuggestions(): void {
    this.wikiSuggestions.set([]);
    this.wikiSuggestionsItemId.set(null);
    if (this.wikiSearchTimer) {
      clearTimeout(this.wikiSearchTimer);
      this.wikiSearchTimer = null;
    }
  }

  /**
   * Handles the user clicking a task checkbox or pressing Ctrl+Enter.
   * Cycles through: normal → task → running → done → normal via cycleTask API.
   */
  async onTaskStatusToggled(item: Item): Promise<void> {
    await this.doCycleTask(item);
  }

  async onStatusCycleRequested(item: Item): Promise<void> {
    await this.doCycleTask(item);
  }

  private async doCycleTask(item: Item): Promise<void> {
    try {
      const originalItem = { ...item };
      const wasTask = item.types?.includes('task');
      const updated = await this.api.cycleTask(item.id);

      // When an item just became a task on a daily journal, set plan to the journal date
      if (!wasTask && updated.types?.includes('task') && this.isDailyJournal(item)) {
        const withPlan = await this.api.upsertItemProperty(item.id, 'plan', 'date', item.name);
        this.state.updateItem(item.id, { types: withPlan.types, properties: withPlan.properties });
        this.undo.begin('cycle-task');
        this.undo.pushUpdate(originalItem, { ...originalItem, types: withPlan.types, properties: withPlan.properties });
        this.undo.commit();
      } else {
        this.state.updateItem(item.id, { types: updated.types, properties: updated.properties });
        this.undo.begin('cycle-task');
        this.undo.pushUpdate(originalItem, { ...originalItem, types: updated.types, properties: updated.properties });
        this.undo.commit();
      }
    } catch (e) {
      console.error('Failed to cycle task:', e);
    }
  }

  private isDailyJournal(item: Item): boolean {
    return item.type === 'journal' && /^\d{4}-\d{2}-\d{2}$/.test(item.name);
  }

  /**
   * Handles the user clicking the collapse/expand bullet or pressing the collapse shortcut.
   * Toggles the collapsed state locally for immediate feedback, then persists to the backend.
   * Reverts the state on error.
   * @param item - The item whose collapsed state was toggled
   */
  async onCollapseToggled(item: Item): Promise<void> {
    try {
      const originalItem = { ...item };
      const newCollapsed = !item.collapsed;
      // Update local state immediately for responsive UI
      this.state.toggleCollapsed(item.id);
      this.state.updateItem(item.id, { collapsed: newCollapsed });
      // Persist to backend
      await this.api.updateItem(item.id, { collapsed: newCollapsed });

      this.undo.begin('collapse');
      this.undo.pushUpdate(originalItem, { ...originalItem, collapsed: newCollapsed });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to toggle collapse:', e);
      // Revert on error
      this.state.toggleCollapsed(item.id);
      this.state.updateItem(item.id, { collapsed: item.collapsed });
    }
  }

  /**
   * Handles the user pasting content into a item editor. Parses the pasted text into items,
   * splits the current item at the cursor, and inserts the parsed items.
   * Supports plain text, tree markdown, and internally copied flat markdown.
   * @param item - The item where the paste occurred
   * @param event - Contains the pasted text, HTML, and cursor position at the time of paste
   */
  async onPaste(item: Item, event: { text: string; html: string; cursorPos: number }): Promise<void> {
    let textToParse = event.text;
    let isPlainText = true;

    // Check 1: If we have stored flat markdown for this clipboard content, use it
    const flatMd = this.clipboard.consume(event.text);
    if (flatMd !== null) {
      textToParse = flatMd;
      isPlainText = false;
    }
    // Check 2: If text looks like tree markdown (starts with "- "), convert it
    else if (event.text.trimStart().startsWith('- ')) {
      try {
        const result = await this.api.importTreeMarkdown(event.text);
        textToParse = result.flatMarkdown;
        isPlainText = false;
      } catch (e) {
        console.error('Failed to import tree markdown, using raw text:', e);
      }
    }

    const skipSplit = isPlainText && hasUnclosedCodeFence(item.body.substring(0, event.cursorPos));
    const parsedItems = skipSplit
      ? [{ body: textToParse, level: 0 }]
      : this.parseTextIntoItems(textToParse);

    if (parsedItems.length === 0) return;

    const originalItem = { ...item };

    // Split existing text at caret position
    const before = item.body.substring(0, event.cursorPos);
    const after = item.body.substring(event.cursorPos);

    // Merge existing text around pasted content
    parsedItems[0].body = before + parsedItems[0].body;
    parsedItems[parsedItems.length - 1].body = parsedItems[parsedItems.length - 1].body + after;

    try {
      this.undo.begin('paste', { itemId: item.id, position: event.cursorPos });

      // First item replaces current editing item
      const firstItem = parsedItems[0];
      await this.saveItemBody(item.id, firstItem.body);
      this.dirtyItemIds.add(item.id);

      const afterFirstItem = this.state.getItemById(item.id);
      if (afterFirstItem) {
        this.undo.pushUpdate(originalItem, { ...afterFirstItem });
      }

      // Add remaining items after the current item
      if (parsedItems.length > 1) {
        let prevItemId = item.id;
        for (let i = 1; i < parsedItems.length; i++) {
          const parsedItem = parsedItems[i];
          const newItem = await this.api.createItem(
            item.pageId,
            prevItemId,
            item.level + parsedItem.level,
            parsedItem.body
          );
          this.state.addItems([newItem], prevItemId);

          const finalNewItem = this.state.getItemById(newItem.id);
          this.undo.pushCreate(finalNewItem ?? newItem);

          prevItemId = newItem.id;
        }
      }

      this.undo.commit();

      // Focus the last created item or the current item
      this.focusEditor();
    } catch (e) {
      console.error('Failed to process paste:', e);
      this.undo.abort();
    }
  }

  async onImagePaste(item: Item, event: { file: File; cursorPos: number }): Promise<void> {
    try {
      const result = await this.api.uploadAsset(event.file);
      const fileName = result.fileName;
      const markdown = `![${fileName}](../assets/${fileName})`;

      const before = item.body.substring(0, event.cursorPos);
      const after = item.body.substring(event.cursorPos);
      const newBody = before + markdown + after;

      await this.saveItemBody(item.id, newBody);
      this.dirtyItemIds.add(item.id);

      // Place cursor after the inserted markdown
      const newPos = event.cursorPos + markdown.length;
      this.state.setFocusedItem(item.id, newPos);
      this.focusEditor(item.id);
    } catch (e) {
      console.error('Failed to paste image:', e);
    }
  }

  private parseTextIntoItems(text: string): Array<{ body: string; level: number }> {
    const items: Array<{ body: string; level: number }> = [];
    const lines = text.split('\n');
    let inCodeFence = false;

    // Current item accumulation
    let currentBodyLines: string[] = [];
    let currentLevel = 1;
    let hasContent = false;

    const flushItem = () => {
      if (!hasContent) return;
      let body = currentBodyLines.join('\n');

      // Whitespace-only body → empty
      if (body.length > 0 && body.trim() === '') body = '';
      items.push({ body, level: currentLevel });
      currentBodyLines = [];
      currentLevel = 1;
      hasContent = false;
    };

    for (const line of lines) {
      // Track code fences
      if (line.trim().startsWith('```')) {
        inCodeFence = !inCodeFence;
      }

      // Blank line outside code fence → flush current item
      if (line === '' && !inCodeFence) {
        flushItem();
        continue;
      }

      // Try to match metadata comments (only outside code fences)
      if (!inCodeFence) {
        const levelMatch = line.match(/^<!-- level=(\d+) -->$/);
        if (levelMatch) {
          currentLevel = parseInt(levelMatch[1], 10);
          hasContent = true;
          continue;
        }

        // Skip other metadata comments
        if (/^<!-- [0-9a-f]{7,8}(-[0-9a-f-]+)? -->$/.test(line) || /^<!-- \w+=.*? -->$/.test(line)) {
          hasContent = true;
          continue;
        }
      }

      // Regular body line
      currentBodyLines.push(line);
      hasContent = true;
    }

    // Flush last item
    flushItem();

    // Normalize levels: subtract the top item's level to get relative offsets (0-based)
    if (items.length > 0) {
      const topLevel = items[0].level;
      for (const item of items) {
        item.level = Math.max(0, item.level - topLevel);
      }
    }

    return items;
  }

  /**
   * Handles the user dragging the mouse beyond the bounds of a item editor.
   * Exits edit mode and enters multi-item selection mode with drag tracking.
   * @param item - The item where the drag started
   * @param direction - Whether the drag went 'above' or 'below' the item
   */
  onDragBeyondBounds(item: Item, direction: 'above' | 'below'): void {
    // Exit edit mode
    this.state.removeAllEditingItems();
    // Enter selection mode
    this.state.setItemSelectionMode(true);
    this.selectionAnchorItemId = item.id;
    this.isSelectionDragging = true;

    // Select anchor item + children
    const group = this.state.getItemWithChildren(item.id);
    const selectedSet = new Set<string>(group.map(b => b.id));
    this.state.setSelectedItemIds(selectedSet);

    // Register document-level listeners
    document.addEventListener('mouseup', this.selectionMouseUpHandler);
    document.addEventListener('keydown', this.selectionKeydownHandler);
    document.addEventListener('copy', this.selectionCopyHandler);
    document.addEventListener('cut', this.selectionCutHandler);
  }

  /**
   * Handles the user's mouse entering a item during a drag selection.
   * Extends the selection range from the anchor item to the hovered item.
   * @param item - The item the mouse entered
   */
  onItemMouseEnter(item: Item): void {
    if (!this.isSelectionDragging) return;
    this.selectionExtentItemId = item.id;

    const visibleItems = this.state.visibleItems();
    const anchorIdx = visibleItems.findIndex(b => b.id === this.selectionAnchorItemId);
    const currentIdx = visibleItems.findIndex(b => b.id === item.id);
    if (anchorIdx < 0 || currentIdx < 0) return;

    const [start, end] = [Math.min(anchorIdx, currentIdx), Math.max(anchorIdx, currentIdx)];
    const selectedSet = new Set<string>();
    for (let i = start; i <= end; i++) {
      // getItemWithChildren uses _items() (all items) so collapsed children are included
      const group = this.state.getItemWithChildren(visibleItems[i].id);
      group.forEach(b => selectedSet.add(b.id));
    }
    this.state.setSelectedItemIds(selectedSet);
  }

  private onSelectionMouseUp(): void {
    this.isSelectionDragging = false;
    document.removeEventListener('mouseup', this.selectionMouseUpHandler);

    // Add click listener to detect clicks outside selection
    // Use setTimeout to avoid the current mouseup from triggering it
    setTimeout(() => {
      document.addEventListener('click', this.selectionClickHandler);
    }, 0);
  }

  private onSelectionClick(e: MouseEvent): void {
    this.exitSelectionMode();
  }

  private handleSelectionKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.exitSelectionMode();
      return;
    }

    // Ctrl+Z / Ctrl+Shift+Z in selection mode
    if ((e.key === 'z' || e.key === 'Z') && e.ctrlKey && !e.altKey) {
      e.preventDefault();
      if (e.shiftKey) {
        this.performRedo();
      } else {
        this.performUndo();
      }
      return;
    }

    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      this.deleteSelectedItems();
      return;
    }

    if (e.shiftKey && e.altKey) {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.moveSelectedItemsUp();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.moveSelectedItemsDown();
        return;
      }
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        this.outdentSelectedItems();
        return;
      }
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        this.indentSelectedItems();
        return;
      }
    }

    if (e.shiftKey && !e.altKey && !e.ctrlKey) {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.shiftExtendUp();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.shiftExtendDown();
        return;
      }
    }

    if (e.key === 'Tab') {
      e.preventDefault();
      if (e.shiftKey) {
        this.outdentSelectedItems();
      } else {
        this.indentSelectedItems();
      }
      return;
    }

    if (e.ctrlKey && e.key === 'c') {
      e.preventDefault();
      this.copySelectedItems();
      return;
    }

    if (e.ctrlKey && e.key === 'x') {
      e.preventDefault();
      this.cutSelectedItems();
      return;
    }
  }

  private getSelectedRootItems(): Item[] {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    const roots: Item[] = [];

    for (const item of allItems) {
      if (!selectedIds.has(item.id)) continue;
      // Check if parent is in selection
      const idx = allItems.indexOf(item);
      let isRoot = true;
      for (let i = idx - 1; i >= 0; i--) {
        if (allItems[i].level < item.level) {
          if (selectedIds.has(allItems[i].id)) {
            isRoot = false;
          }
          break;
        }
      }
      if (isRoot) roots.push(item);
    }
    return roots;
  }

  private getOrderedSelectedItemIds(): string[] {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    return allItems.filter(b => selectedIds.has(b.id)).map(b => b.id);
  }

  private async deleteSelectedItems(): Promise<void> {
    const selectedIds = this.getOrderedSelectedItemIds();
    if (selectedIds.length === 0) return;

    // Snapshot before deletion
    const deletedItems = selectedIds.map(id => this.state.getItemById(id)).filter((b): b is Item => !!b);

    this.exitSelectionMode();

    this.undo.begin('delete-selection');
    for (const b of deletedItems) {
      this.undo.pushDelete({ ...b });
    }
    this.undo.commit();

    // Delete from state first for responsiveness
    this.state.deleteItems(selectedIds);

    // Delete from backend
    try {
      await Promise.all(selectedIds.map(id => this.api.deleteItem(id)));
    } catch (e) {
      console.error('Failed to delete selected items:', e);
    }
  }

  private async moveSelectedItemsUp(): Promise<void> {
    const rootItems = this.getSelectedRootItems();
    if (rootItems.length === 0) return;

    const firstRoot = rootItems[0];

    const target = findMoveUpTarget(this.state.items(), firstRoot.id);
    if (!target) return;

    const allSelectedIds: string[] = [];
    for (const root of rootItems) {
      const group = this.state.getItemWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }
    const beforeSnapshots = allSelectedIds.map(id => {
      const b = this.state.getItemById(id);
      return b ? { ...b } : null;
    }).filter((b): b is Item => !!b);

    try {
      const moved = await this.api.moveItems(allSelectedIds, firstRoot.pageId, target.afterItemId);
      this.state.replaceItems(allSelectedIds, moved, target.afterItemId);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error('Failed to move selected items up:', e);
    }
  }

  private async moveSelectedItemsDown(): Promise<void> {
    const rootItems = this.getSelectedRootItems();
    if (rootItems.length === 0) return;

    const lastRoot = rootItems[rootItems.length - 1];

    const target = findMoveDownTarget(this.state.items(), lastRoot.id);
    if (!target) return;

    const allSelectedIds: string[] = [];
    for (const root of rootItems) {
      const group = this.state.getItemWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }
    const beforeSnapshots = allSelectedIds.map(id => {
      const b = this.state.getItemById(id);
      return b ? { ...b } : null;
    }).filter((b): b is Item => !!b);

    try {
      const moved = await this.api.moveItems(allSelectedIds, lastRoot.pageId, target.afterItemId);
      this.state.replaceItems(allSelectedIds, moved, target.afterItemId);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error('Failed to move selected items down:', e);
    }
  }

  private async indentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    const itemsToIndent = allItems.filter(b => selectedIds.has(b.id));

    if (itemsToIndent.length === 0) return;
    const firstIdx = allItems.findIndex(b => b.id === itemsToIndent[0].id);
    if (firstIdx <= 0) return;

    this.undo.begin('indent-selection');
    for (const b of itemsToIndent) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + 1 });
      this.state.updateItem(b.id, { level: b.level + 1 });
    }
    this.undo.commit();

    try {
      const results = await Promise.all(itemsToIndent.map(b => this.api.updateItem(b.id, { level: b.level + 1 })));
      for (const r of results) {
        this.state.updateItem(r.id, { idx: r.idx, level: r.level });
      }
    } catch (e) {
      console.error('Failed to indent selected items:', e);
    }
  }

  private async outdentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    const itemsToOutdent = allItems.filter(b => selectedIds.has(b.id));

    if (itemsToOutdent.some(b => b.level <= 1)) return;

    this.undo.begin('outdent-selection');
    for (const b of itemsToOutdent) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level - 1 });
      this.state.updateItem(b.id, { level: b.level - 1 });
    }
    this.undo.commit();

    try {
      const results = await Promise.all(itemsToOutdent.map(b => this.api.updateItem(b.id, { level: b.level - 1 })));
      for (const r of results) {
        this.state.updateItem(r.id, { idx: r.idx, level: r.level });
      }
    } catch (e) {
      console.error('Failed to outdent selected items:', e);
    }
  }

  private async copySelectedItems(): Promise<void> {
    const orderedIds = this.getOrderedSelectedItemIds();
    if (orderedIds.length === 0) return;

    try {
      const result = await this.api.exportTreeMarkdown(orderedIds);

      // Write to system clipboard
      const clipboardItem = new ClipboardItem({
        'text/plain': new Blob([result.treeMarkdown], { type: 'text/plain' }),
        'text/html': new Blob([renderToHtml(result.ast)], { type: 'text/html' }),
      });
      await navigator.clipboard.write([clipboardItem]);

      // Store flat markdown in memory
      this.clipboard.store(result.flatMarkdown, result.treeMarkdown);
    } catch (e) {
      console.error('Failed to copy selected items:', e);
    }
  }

  private async cutSelectedItems(): Promise<void> {
    await this.copySelectedItems();
    await this.deleteSelectedItems();
  }

  private onSelectionCopy(e: ClipboardEvent): void {
    if (!this.state.itemSelectionMode()) return;
    e.preventDefault();
    this.copySelectedItems();
  }

  private onSelectionCut(e: ClipboardEvent): void {
    if (!this.state.itemSelectionMode()) return;
    e.preventDefault();
    this.cutSelectedItems();
  }

  async performUndo(): Promise<void> {
    if (this.undo.isPending()) return;

    // Cancel pending auto-save for the editing item
    const editingId = this.state.focusedItemId();
    if (editingId && this.dirtyItemIds.has(editingId)) {
      this.dirtyItemIds.delete(editingId);
      this.autoSave.cancelItem(editingId);
    }

    const entry = this.undo.popUndo();
    if (!entry) return;

    // Apply inverse ops in reverse order
    for (let i = entry.ops.length - 1; i >= 0; i--) {
      const op = entry.ops[i];
      switch (op.type) {
        case 'update':
          this.state.updateItem(op.before.id, op.before);
          if (op.before.collapsed !== op.after.collapsed) {
            this.state.toggleCollapsed(op.before.id);
          }
          break;
        case 'create':
          this.state.deleteItem(op.item.id);
          break;
        case 'delete':
          this.state.addItem(op.item, this.findInsertionPoint(op.item));
          break;
      }
    }

    // Restore cursor
    if (entry.cursorBefore) {
      this.state.addEditingItem(entry.cursorBefore.itemId);
      this.state.setFocusedItem(entry.cursorBefore.itemId, entry.cursorBefore.position);
      this.focusEditor(entry.cursorBefore.itemId);
    } else {
      // For body edits and other entries without explicit cursor, keep editing the first updated item
      const firstUpdate = entry.ops.find(op => op.type === 'update');
      if (firstUpdate && firstUpdate.type === 'update') {
        const pos = firstUpdate.before.body.length;
        this.state.addEditingItem(firstUpdate.before.id);
        this.state.setFocusedItem(firstUpdate.before.id, pos);
        this.focusEditor(firstUpdate.before.id);
      } else {
        this.state.removeAllEditingItems();
      }
    }

    this.undo.pushRedo(entry);
    this.syncToServer(entry, 'undo');
  }

  async performRedo(): Promise<void> {
    if (this.undo.isPending()) return;

    // Cancel pending auto-save for the editing item
    const editingId = this.state.focusedItemId();
    if (editingId && this.dirtyItemIds.has(editingId)) {
      this.dirtyItemIds.delete(editingId);
      this.autoSave.cancelItem(editingId);
    }

    const entry = this.undo.popRedo();
    if (!entry) return;

    // Apply forward ops in order
    for (const op of entry.ops) {
      switch (op.type) {
        case 'update':
          this.state.updateItem(op.after.id, op.after);
          if (op.before.collapsed !== op.after.collapsed) {
            this.state.toggleCollapsed(op.after.id);
          }
          break;
        case 'create':
          this.state.addItem(op.item, this.findInsertionPoint(op.item));
          break;
        case 'delete':
          this.state.deleteItem(op.item.id);
          break;
      }
    }

    // Restore cursor
    if (entry.cursorAfter) {
      this.state.addEditingItem(entry.cursorAfter.itemId);
      this.state.setFocusedItem(entry.cursorAfter.itemId, entry.cursorAfter.position);
      this.focusEditor(entry.cursorAfter.itemId);
    } else {
      const firstUpdate = entry.ops.find(op => op.type === 'update');
      if (firstUpdate && firstUpdate.type === 'update') {
        const pos = firstUpdate.after.body.length;
        this.state.addEditingItem(firstUpdate.after.id);
        this.state.setFocusedItem(firstUpdate.after.id, pos);
        this.focusEditor(firstUpdate.after.id);
      } else {
        this.state.removeAllEditingItems();
      }
    }

    this.undo.pushUndo(entry);
    this.syncToServer(entry, 'redo');
  }

  private findInsertionPoint(item: Item): string {
    const items = this.state.items();
    let bestId = '';
    for (const b of items) {
      if (b.pageId === item.pageId && b.idx < item.idx) {
        bestId = b.id;
      }
    }
    return bestId;
  }

  private async syncToServer(entry: UndoEntry, dir: 'undo' | 'redo'): Promise<void> {
    try {
      const ops = dir === 'undo' ? [...entry.ops].reverse() : entry.ops;
      for (const op of ops) {
        if (op.type === 'update') {
          const target = dir === 'undo' ? op.before : op.after;
          const source = dir === 'undo' ? op.after : op.before;
          const updated = await this.api.updateItem(target.id, {
            body: target.body,
            level: target.level,
            collapsed: target.collapsed,
          });
          this.state.updateItem(updated.id, { idx: updated.idx, level: updated.level });
        } else if (op.type === 'create') {
          if (dir === 'undo') {
            // Undo a create = delete
            await this.api.deleteItem(op.item.id);
          } else {
            // Redo a create = recreate
            const afterId = this.findInsertionPoint(op.item);
            const newItem = await this.api.createItem(
              op.item.pageId, afterId, op.item.level, op.item.body
            );
            // Remap: update local state with real server item
            this.state.deleteItem(op.item.id);
            this.state.addItem(newItem, afterId);
            op.item = newItem;
          }
        } else if (op.type === 'delete') {
          if (dir === 'undo') {
            // Undo a delete = recreate
            const afterId = this.findInsertionPoint(op.item);
            const newItem = await this.api.createItem(
              op.item.pageId, afterId, op.item.level, op.item.body
            );
            // Remap
            this.state.deleteItem(op.item.id);
            this.state.addItem(newItem, afterId);
            op.item = newItem;
          } else {
            // Redo a delete = delete
            await this.api.deleteItem(op.item.id);
          }
        }
      }
    } catch (e) {
      console.error('Failed to sync undo/redo to server:', e);
    }
  }

  private exitSelectionMode(): void {
    this.state.clearSelection();
    this.selectionAnchorItemId = null;
    this.selectionExtentItemId = null;
    this.isSelectionDragging = false;
    document.removeEventListener('mouseup', this.selectionMouseUpHandler);
    document.removeEventListener('keydown', this.selectionKeydownHandler);
    document.removeEventListener('click', this.selectionClickHandler);
    document.removeEventListener('copy', this.selectionCopyHandler);
    document.removeEventListener('cut', this.selectionCutHandler);
  }

  /** Cleans up document-level event listeners and wiki suggestion state on component destruction. */
  ngOnDestroy(): void {
    document.removeEventListener('mouseup', this.selectionMouseUpHandler);
    document.removeEventListener('keydown', this.selectionKeydownHandler);
    document.removeEventListener('click', this.selectionClickHandler);
    document.removeEventListener('copy', this.selectionCopyHandler);
    document.removeEventListener('cut', this.selectionCutHandler);
    this.closeWikiSuggestions();
  }

  private async saveItemBody(itemId: string, body: string): Promise<Item> {
    const normalized = ensureCodeFenceClosed(body);
    this.state.updateItem(itemId, { body: normalized });
    const updated = await this.api.updateItem(itemId, { body: normalized });
    this.state.updateItem(itemId, { ast: updated.ast });
    return updated;
  }

  private focusEditor(itemId?: string, callback?: () => void): void {
    afterNextRender(() => {
      const selector = itemId
        ? `#item-editor-${itemId}`
        : 'app-item-editor textarea';
      const el = document.querySelector<HTMLTextAreaElement>(selector);
      if (el) {
        el.focus({ preventScroll: true });
        el.style.height = 'auto';
        el.style.height = el.scrollHeight + 'px';
      }
      callback?.();
    }, { injector: this.injector });
  }
}
