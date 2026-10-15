import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild, viewChildren, signal, computed, untracked, OnDestroy } from '@angular/core';
import { Subscription } from 'rxjs';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { ScrollingModule as ExperimentalScrollingModule } from '@angular/cdk-experimental/scrolling';
import { OutlineItemComponent } from './outline-item';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { Item } from '../../core/models/item.model';
import { ClipboardService } from '../../core/services/clipboard.service';
import { UndoService } from '../../core/services/undo.service';
import { UndoReplayService } from './undo-replay.service';
import { formatJournalDate, shiftIsoDate } from '../../core/utils/date.utils';
import { classifyJournalFileName } from '../../core/utils/journal-filename.utils';
import { NavigationService } from '../../core/services/navigation.service';
import { ConfigService } from '../../core/services/config.service';
import { WikiLinkContext, hasUnclosedCodeFence } from './item-editor';
import { ItemEditService } from './item-edit.service';
import { findCrossPagePlacement, findMoveDownTarget, findMoveUpTarget, getSubtree } from './move-utils';
import {
  EditContext, TransclusionContext, canIndent, editContextOf, hasSiblingBelow, idsOfRangeWithSubtrees,
  lastDescendantOutside, levelForNewItemAfter, parentIndexOf, positionAfterSubtree, positionBefore,
  selectedRootItems, selectionStartIndex, transclusionContextOf, transclusionScopedItems,
} from './outline-tree';
import { renderToHtml } from 'flatmarkdown-ast2html';
import { HmtidService } from '../../core/services/hmtid.service';
import { SseService } from '../../core/services/sse.service';

type SelectionEventName = 'keydown' | 'click' | 'copy' | 'cut' | 'mouseup';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, ExperimentalScrollingModule, OutlineItemComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport autosize [minBufferPx]="500" [maxBufferPx]="1000" class="outline-viewport">
      <div
        *cdkVirtualFor="let item of state.visibleItems(); let i = index; trackBy: trackById"
        class="outline-item"
        (mouseenter)="onItemMouseEnter(item)"
        (wheel)="onItemWheel(item, $event)"
      >
        @if (isFirstItemOfDate(i) && item.name !== currentPageDate() && item.type === 'journal') {
          <div class="date-header">
            {{ formatDateHeader(item.name) }}
          </div>
        }
        <app-outline-item
          #outlineItem
          [item]="item"
          [effectiveZoom]="effectiveZoomMap().get(item.id) ?? 1"
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
          (taskToggled)="onTaskCycleRequested(item)"
          (statusCycleRequested)="onTaskCycleRequested(item)"
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
          (pasteAsTransclusion)="onPasteAsTransclusion(item)"
          (removeTransclusion)="onRemoveTransclusion(item)"
          (sendToArchive)="onSendToArchive(item)"
          (resetZoom)="onResetZoom(item)"
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
      padding-right: 20px;
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
  private readonly injector = inject(Injector);
  private readonly clipboard = inject(ClipboardService);
  private readonly nav = inject(NavigationService);
  private readonly undo = inject(UndoService);
  private readonly undoReplay = inject(UndoReplayService);
  private readonly edits = inject(ItemEditService);
  private readonly config = inject(ConfigService);
  private readonly hmtid = inject(HmtidService);
  private readonly sse = inject(SseService);
  private sseSub?: Subscription;
  private scrollSub?: Subscription;

  readonly pageId = input.required<string>();
  readonly focusItemId = input<string | null | undefined>('');
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic item loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  readonly crossDayMoved = output<{ targetDate: string; navigate: boolean; itemId: string }>();  // Emits after cross-day move
  readonly topItemChanged = output<string>();  // Emits name of top visible item
  readonly scrolledFromTop = output<boolean>();  // Emits true when scrolled away from top

  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');
  private readonly outlineItems = viewChildren<OutlineItemComponent>('outlineItem');

  // ---- Editing state ----

  /** Pages changed on disk while the user was editing; reloaded once editing stops. */
  private readonly pendingFileReloads = new Set<string>();
  /** Tail of the move queue; see enqueueMove. */
  private moveQueue: Promise<void> = Promise.resolve();

  // ---- Wiki link suggestions ----

  readonly wikiSuggestions = signal<string[]>([]);
  readonly wikiSuggestionIndex = signal(0);
  readonly wikiSuggestionsItemId = signal<string | null>(null);
  readonly wikiSuggestionsPos = signal({ x: 0, y: 0 });
  private wikiSearchTimer: ReturnType<typeof setTimeout> | null = null;

  // ---- Multi-item selection ----

  /** The item the selection started from. */
  private selectionAnchorItemId: string | null = null;
  /** The item the selection was last extended to. */
  private selectionExtentItemId: string | null = null;
  /** True while the mouse button is held after dragging out of an editor. */
  private isSelectionDragging = false;
  /** Document-level listeners that are active only in selection mode. */
  private readonly selectionListeners: { [K in SelectionEventName]: (e: DocumentEventMap[K]) => void } = {
    keydown: e => this.handleSelectionKeydown(e),
    click: () => this.exitSelectionMode(),
    copy: e => this.onSelectionCopy(e),
    cut: e => this.onSelectionCut(e),
    mouseup: () => this.onSelectionMouseUp(),
  };

  // Effective zoom map: item ID → cumulative zoom (product of own + ancestor zooms)
  readonly effectiveZoomMap = computed(() => {
    const items = this.state.visibleItems();
    const map = new Map<string, number>();
    // Stack of [level, cumulativeZoom] for ancestor tracking
    const stack: { level: number; zoom: number }[] = [];
    for (const item of items) {
      // Pop stack entries that are not ancestors of this item
      while (stack.length > 0 && stack[stack.length - 1].level >= item.level) {
        stack.pop();
      }
      const parentZoom = stack.length > 0 ? stack[stack.length - 1].zoom : 1;
      const ownZoom = parseFloat(item.properties['zoom'] as string) || 1;
      const effective = parentZoom * ownZoom;
      map.set(item.id, effective);
      stack.push({ level: item.level, zoom: effective });
    }
    return map;
  });

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

    // Changes picked up from the markdown folder: refresh the affected page's items
    this.sseSub = this.sse.pagesUpdated$.subscribe(ev => {
      if (ev.source === 'file') this.reloadPageFromFile(ev.pageId);
    });
    effect(() => {
      if (this.state.editingItemIds().size === 0 && this.pendingFileReloads.size > 0) {
        // Give the auto-save a moment to flush before replacing items
        setTimeout(() => this.flushPendingFileReloads(), 1500);
      }
    });

    // Subscribe to viewport scroll outside Angular change detection
    effect(() => {
      const vp = this.viewport();
      if (!vp) return;
      untracked(() => {
        this.scrollSub?.unsubscribe();
        this.scrollSub = vp.elementScrolled().subscribe(() => this.onViewportScroll());
      });
    });

    // Handle scroll-to-item requests from sidebar TOC
    effect(() => {
      const itemId = this.state.scrollRequestItemId();
      if (!itemId) return;
      untracked(() => {
        this.state.clearScrollRequest();
        this.scrollToItemElement(itemId, 'smooth');
      });
    });

    // Handle pending edit requests (e.g. after cross-day move)
    effect(() => {
      const itemId = this.state.pendingEditItemId();
      if (!itemId) return;
      untracked(() => {
        this.state.clearPendingEditItem();
        this.state.addEditingItem(itemId);
        this.state.setFocusedItem(itemId);
        this.focusEditor(itemId);
      });
    });
  }

  ngOnDestroy(): void {
    this.scrollSub?.unsubscribe();
    this.sseSub?.unsubscribe();
    this.stopListeningForSelectionEvents();
    this.closeWikiSuggestions();
  }

  onViewportScroll(): void {
    const vp = this.viewport();
    if (!vp) return;
    const offset = vp.measureScrollOffset();
    const range = vp.getRenderedRange();
    const items = this.state.visibleItems();
    const totalItems = items.length;

    this.scrolledFromTop.emit(offset > 0);

    // Find the actual first visible item using DOM positions.
    // range.start includes buffer items above the viewport, so we inspect
    // rendered elements to find the one truly at the viewport top.
    const vpEl = vp.elementRef.nativeElement;
    const vpTop = vpEl.getBoundingClientRect().top;
    const wrapper = vpEl.querySelector('.cdk-virtual-scroll-content-wrapper');
    if (wrapper) {
      for (let ci = 0; ci < wrapper.children.length; ci++) {
        const el = wrapper.children[ci] as HTMLElement;
        if (el.getBoundingClientRect().bottom > vpTop) {
          const idx = range.start + ci;
          // Walk backward to find the owning journal page item
          for (let j = Math.min(idx, totalItems - 1); j >= 0; j--) {
            if (this.state.isOwnItem(items[j].id)) {
              this.topItemChanged.emit(items[j].name);
              break;
            }
          }
          break;
        }
      }
    }

    // Check if we need more buffer (100 items ahead)
    if (totalItems < range.end + 100 && totalItems > 0) {
      const lastItem = items[totalItems - 1];
      const filepath = lastItem.dirPath + lastItem.name;
      this.loadMore.emit({ filepath, currentIndex: range.start });
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
    // Skip transcluded descendants — they belong to a different page but are
    // displayed inline, so they should never trigger a date header.
    if (!this.state.isOwnItem(currentItem.id)) return false;
    // Find the previous item that is actually on our page (skip transcluded descendants)
    for (let i = index - 1; i >= 0; i--) {
      if (this.state.isOwnItem(items[i].id)) {
        return currentItem.name !== items[i].name;
      }
    }
    return true;
  }

  /**
   * Formats a journal file name into a human-readable date string for display as a date header.
   * @param fileName - The journal file name to format
   * @returns A formatted date string
   */
  formatDateHeader(fileName: string): string {
    return formatJournalDate(fileName);
  }

  // ---- Loading ----

  /**
   * Replaces the items of one page with the server's current version after the
   * markdown file changed on disk. Waits while the user is editing so their typing
   * is not overwritten.
   */
  private async reloadPageFromFile(pageId: string): Promise<void> {
    const current = this.state.items();
    if (!current.some(i => i.pageId === pageId)) return;  // not shown in this outline
    if (this.edits.isEditInProgress()) {
      this.pendingFileReloads.add(pageId);
      return;
    }
    try {
      const result = await this.api.getPageById(pageId);
      const items = this.state.items();
      const first = items.findIndex(i => i.pageId === pageId);
      if (first < 0) return;
      const before = items.slice(0, first);
      const after = items.slice(first).filter(i => i.pageId !== pageId);
      this.state.setItems([...before, ...result.items, ...after]);
      if (result.transcludedTrees) {
        this.state.mergeTranscludedTrees(result.transcludedTrees);
      }
    } catch (e) {
      console.error('Failed to reload page from file:', pageId, e);
    }
  }

  private flushPendingFileReloads(): void {
    if (this.edits.isEditInProgress()) return;
    const ids = [...this.pendingFileReloads];
    this.pendingFileReloads.clear();
    for (const id of ids) this.reloadPageFromFile(id);
  }

  private async loadItems(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPageById(pageId);
      this.state.setItems(result.items);
      if (result.transcludedTrees) {
        this.state.setTranscludedTrees(result.transcludedTrees);
      }
      this.scrollToFocusItem();
    } catch (e) {
      console.error('Failed to load items for page:', pageId, e);
    }
  }

  private async ensureAtLeastOneItem(): Promise<void> {
    if (this.state.items().length > 0) return;
    const newItem = await this.api.createItem(this.pageId(), 'first', 1, '');
    this.state.addItem(newItem, 'first');
    this.state.setFocusedItem(newItem.id);
    this.focusEditor(newItem.id);
  }

  // ---- Scrolling ----

  private scrollToFocusItem(): void {
    const itemId = this.focusItemId();
    if (!itemId) return;
    const items = this.state.visibleItems();
    const index = items.findIndex(b => b.id === itemId);
    if (index < 0) return;
    const subtreeIds = this.state.getChildren(itemId).map(b => b.id);
    this.enterSelectionMode(itemId, new Set([itemId, ...subtreeIds]));
    setTimeout(() => this.scrollToItemElement(itemId), 50);
  }

  private scrollToItemElement(itemId: string, behavior: ScrollBehavior = 'auto'): void {
    const el = document.getElementById(itemId);
    if (el) {
      this.scrollToElementAtFraction(el, behavior, 0.1);
      return;
    }
    const vp = this.viewport();
    if (!vp) return;
    const items = this.state.visibleItems();
    const targetIndex = items.findIndex(b => b.id === itemId);
    if (targetIndex < 0) return;

    // Estimate position and jump
    const range = vp.getRenderedRange();
    const renderedCount = range.end - range.start;
    const avgItemSize = renderedCount > 0
      ? vp.measureRenderedContentSize() / renderedCount
      : 40;
    const vpHeight = vp.getViewportSize();
    const estimatedOffset = Math.max(0, targetIndex * avgItemSize - vpHeight * 0.1);
    vp.scrollToOffset(estimatedOffset);

    // After jump, iteratively scroll until item appears
    this.scrollSearchStep(itemId, targetIndex, 0);
  }

  private scrollSearchStep(itemId: string, targetIndex: number, attempt: number): void {
    if (attempt > 50) return;
    setTimeout(() => {
      const el = document.getElementById(itemId);
      if (el) {
        this.scrollToElementAtFraction(el, 'smooth', 0.1);
        return;
      }
      const vp = this.viewport();
      if (!vp) return;
      const range = vp.getRenderedRange();
      const vpHeight = vp.getViewportSize();
      if (targetIndex >= range.end) {
        vp.scrollToOffset(vp.measureScrollOffset() + vpHeight * 0.8);
      } else if (targetIndex < range.start) {
        vp.scrollToOffset(Math.max(0, vp.measureScrollOffset() - vpHeight * 0.8));
      }
      this.scrollSearchStep(itemId, targetIndex, attempt + 1);
    }, 50);
  }

  private scrollToElementAtFraction(el: HTMLElement, behavior: ScrollBehavior, fraction: number): void {
    const scrollParent = el.closest('cdk-virtual-scroll-viewport') as HTMLElement;
    if (!scrollParent) { el.scrollIntoView({ behavior, block: 'start' }); return; }
    const elRect = el.getBoundingClientRect();
    const vpRect = scrollParent.getBoundingClientRect();
    const targetY = vpRect.top + vpRect.height * fraction;
    const delta = elRect.top - targetY;
    scrollParent.scrollBy({ top: delta, behavior });
  }

  // ---- Editing ----

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
        await this.edits.handOverEdit(() => this.edits.saveItemIfDirty(current));
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
    this.edits.recordTyping(item, newBody);
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
    await this.edits.finishEdit(item);
  }

  /**
   * Focuses the editor of an item once the next render is done.
   * @param itemId - The item whose editor is focused; the first editor on the page when omitted
   * @returns A promise that settles after the editor was focused
   */
  private focusEditor(itemId?: string): Promise<void> {
    return new Promise(resolve => {
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
        resolve();
      }, { injector: this.injector });
    });
  }

  // ---- Enter, Backspace, Delete ----

  /**
   * Handles the user pressing Enter in a item editor. Splits the item at the cursor position,
   * creating a new item with the text after the cursor. If the item is empty and indented,
   * outdents instead. If the cursor is at the beginning, inserts an empty item before.
   * @param item - The item where Enter was pressed
   * @param event - Contains the cursor position at the time of the keypress
   */
  async onEnterPressed(item: Item, event: { cursorPos: number }): Promise<void> {
    this.edits.discardPendingEdit(item);

    if (item.body.length === 0 && item.level > 1 && !hasSiblingBelow(this.state.visibleItems(), item.id)) {
      // Empty last child: outdent instead of creating a new item
      await this.onOutdent(item);
      this.focusEditor(item.id);
      return;
    }

    const { cursorPos } = event;
    await this.edits.handOverEdit(async () => {
      if (cursorPos === 0 && item.body.length > 0) {
        await this.insertEmptyItemBefore(item, cursorPos);
      } else if (cursorPos === item.body.length) {
        await this.appendEmptyItemAfter(item, cursorPos);
      } else {
        await this.splitItemAt(item, cursorPos);
      }
    });
  }

  /** Enter at the beginning: a new empty item goes before the current one, which keeps its text. */
  private async insertEmptyItemBefore(item: Item, cursorPos: number): Promise<void> {
    const ctx = this.editContextOf(item);
    // Inside a transclusion the new item goes to the top of the source page
    const position: ItemPosition = ctx.inTransclusion ? 'first' : positionBefore(this.state.items(), item.id);
    const [newItem] = await Promise.all([
      this.api.createItem(ctx.pageId, position, item.level - ctx.levelOffset, ''),
      this.edits.saveBody(this.edits.saveTargetId(item), item.body),
    ]);
    await this.showCreatedItem(newItem, position, ctx);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushCreate(newItem);
    this.undo.commit({ itemId: newItem.id, position: 0 });

    this.state.addEditingItem(newItem.id);
    this.focusEditor(newItem.id);
    this.state.removeEditingItem(item.id);
  }

  /**
   * Enter at the end: the new item is shown at once with a client-side ID and
   * created on the server in the background.
   */
  private async appendEmptyItemAfter(item: Item, cursorPos: number): Promise<void> {
    const ctx = this.editContextOf(item);
    const newLevel = this.levelForNewItemAfter(item, ctx);
    const position = positionAfterSubtree(this.state.items(), item, this.edits.saveTargetId(item));
    const newId = this.hmtid.nextId();
    const optimisticItem: Item = {
      id: newId,
      pageId: ctx.pageId,
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
    this.api.createItem(ctx.pageId, position, newLevel - ctx.levelOffset, '', newId).then(async serverItem => {
      if (ctx.inTransclusion) {
        await this.loadItems(this.pageId());
      } else {
        this.state.updateItem(newId, { idx: serverItem.idx });
      }
    });
    this.state.addItem(optimisticItem, position);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate({ ...item }, { ...item });
    this.undo.pushCreate(optimisticItem);
    this.undo.commit({ itemId: newId, position: 0 });

    this.edits.discardPendingEdit(item);
    this.state.addEditingItem(newId);
    this.state.setFocusedItem(newId);
    this.focusEditor(newId);

    await this.edits.saveBody(this.edits.saveTargetId(item), item.body.substring(0, cursorPos));
    this.state.removeEditingItem(item.id);
  }

  /** Enter in the middle: the text after the cursor moves to a new item below. */
  private async splitItemAt(item: Item, cursorPos: number): Promise<void> {
    const ctx = this.editContextOf(item);
    const newLevel = this.levelForNewItemAfter(item, ctx);
    const position = positionAfterSubtree(this.state.items(), item, this.edits.saveTargetId(item));
    const beforeText = item.body.substring(0, cursorPos);
    const afterText = item.body.substring(cursorPos);
    const originalItem = { ...item };

    const [updatedOriginal, newItem] = await Promise.all([
      this.edits.saveBody(this.edits.saveTargetId(item), beforeText),
      this.api.createItem(ctx.pageId, position, newLevel - ctx.levelOffset, afterText),
    ]);
    await this.showCreatedItem(newItem, position, ctx);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate(originalItem, updatedOriginal);
    this.undo.pushCreate(newItem);
    this.undo.commit({ itemId: newItem.id, position: 0 });

    this.state.addEditingItem(newItem.id);
    this.state.setFocusedItem(newItem.id);
    this.focusEditor(newItem.id);
    this.state.removeEditingItem(item.id);
  }

  /** Puts a newly created item into the state. Inside a transclusion the page is reloaded so the trees are fresh. */
  private async showCreatedItem(newItem: Item, position: ItemPosition, ctx: EditContext): Promise<void> {
    if (ctx.inTransclusion) {
      await this.loadItems(this.pageId());
    } else {
      this.state.addItem(newItem, position);
    }
  }

  /** The shown level of a new item after this one; a transclusion reads the neighbour from the visible list. */
  private levelForNewItemAfter(item: Item, ctx: EditContext): number {
    return levelForNewItemAfter(ctx.inTransclusion ? this.state.visibleItems() : this.state.items(), item);
  }

  private editContextOf(item: Item): EditContext {
    return editContextOf(item, this.state.ownItemIds(), this.state.transcludedTrees());
  }

  /**
   * Handles the user pressing Backspace at the start of a item.
   * Merges the current item's content into the previous item and deletes the current item.
   * @param item - The item where Backspace was pressed at position 0
   */
  async onBackspaceAtStart(item: Item): Promise<void> {
    // Prevent merge on transclusion items
    if (item.transcludeItemId) return;

    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx <= 0) return;

    const prevItem = items[idx - 1];

    // In journal daily pages, prevent merging across page boundaries
    if (this.isDailyJournalItem(item) && prevItem.pageId !== item.pageId) return;

    this.edits.discardPendingEdit(item);
    const originalPrev = { ...prevItem };
    const originalItem = { ...item };
    const cursorPos = prevItem.body.length;
    const mergedBody = prevItem.body + item.body;
    this.state.updateItem(prevItem.id, { body: mergedBody });
    this.state.deleteItem(item.id);
    this.state.addEditingItem(prevItem.id);
    this.focusEditor(prevItem.id);

    const updatedPrev = await this.edits.saveBody(prevItem.id, mergedBody);
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
    // Same limits as Backspace: a transclusion is not merged away, and journal days are not merged
    if (nextItem.transcludeItemId) return;
    if (this.isDailyJournalItem(item) && nextItem.pageId !== item.pageId) return;

    this.edits.discardPendingEdit(nextItem);

    const originalItem = { ...item };
    const originalNext = { ...nextItem };
    const cursorPos = item.body.length;
    const mergedBody = item.body + nextItem.body;
    this.state.updateItem(item.id, { body: mergedBody });
    this.state.deleteItem(nextItem.id);

    const updatedItem = await this.edits.saveBody(item.id, mergedBody);
    await this.api.deleteItem(nextItem.id);

    this.undo.begin('delete', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate(originalItem, updatedItem);
    this.undo.pushDelete(originalNext);
    this.undo.commit({ itemId: item.id, position: cursorPos });
  }

  // ---- Navigation between items ----

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
      await this.navigateTo(item, items[idx - 1], cursorPos);
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
      await this.navigateTo(item, items[idx + 1], cursorPos);
    }
  }

  /** Saves the item being edited and opens the editor of another item. */
  private async navigateTo(from: Item, to: Item, cursorPos: number): Promise<void> {
    await this.edits.saveItemIfDirty(from);
    this.state.addEditingItem(to.id);
    this.state.setFocusedItem(to.id, cursorPos);
    this.state.removeEditingItem(from.id);
  }

  // ---- Indent and outdent ----

  /**
   * Handles the user pressing Tab to indent a item and its children by one level.
   * Validates placement constraints before allowing the indent.
   * @param item - The item to indent
   */
  async onIndent(item: Item): Promise<void> {
    const items = this.state.isOwnItem(item.id) ? this.state.items() : this.state.visibleItems();
    if (!canIndent(items, item.id)) return;

    const tCtx = this.getTransclusionContext(item);
    if (tCtx) {
      await this.shiftLevels(this.getVisibleSubtree(item.id), 1, 'indent', tCtx.levelOffset);
      return;
    }
    await this.shiftLevels(this.state.getItemWithChildren(item.id), 1, 'indent');
  }

  /**
   * Handles the user pressing Shift+Tab to outdent an item and its children by one level.
   *
   * - If the previous item is at level-1, outdent in place (both modes).
   * - Otherwise, find the parent (an item above at level-1):
   *   - logicalOutdent=true: move group to after parent's last child, then outdent.
   *     Siblings below stay with the original parent.
   *   - logicalOutdent=false: outdent in place. Siblings below don't move but
   *     their parent effectively changes to the outdented item.
   *
   * @param item - The item to outdent
   */
  async onOutdent(item: Item): Promise<void> {
    if (item.level <= 1) return;

    // For transcluded descendants, prevent outdenting beyond transclusion root level
    const tCtx = this.getTransclusionContext(item);
    if (tCtx && item.level <= tCtx.rootDisplayLevel + 1) return; // Can't go to same level or above root

    if (tCtx && !this.state.isOwnItem(item.id)) {
      // Transcluded descendant: simple in-place outdent on source page
      await this.shiftLevels(this.getVisibleSubtree(item.id), -1, 'outdent', tCtx.levelOffset);
      return;
    }

    const items = this.state.items();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx < 0) return;
    const prevItem = idx > 0 ? items[idx - 1] : undefined;
    const parentIdx = parentIndexOf(items, idx);
    if (parentIdx < 0) return; // No parent found, cannot outdent

    const group = this.state.getItemWithChildren(item.id);
    const needsMove = !prevItem || prevItem.level !== item.level - 1;

    if (needsMove && this.config.logicalOutdent()) {
      await this.outdentBelowParent(item, group, items[parentIdx]);
    } else {
      // In-place outdent: decrease level of group
      await this.shiftLevels(group, -1, 'outdent');
    }
  }

  /**
   * Logical outdent: the group is placed after the parent's last child and then outdented,
   * so the siblings below it stay with the original parent.
   */
  private async outdentBelowParent(item: Item, group: Item[], parent: Item): Promise<void> {
    const groupIds = group.map(b => b.id);
    const lastChildId = lastDescendantOutside(this.state.items(), parent.id, new Set(groupIds));
    const beforeSnapshots = group.map(b => ({ ...b }));

    for (const b of group) {
      this.state.updateItem(b.id, { level: b.level - 1 });
    }
    await this.api.shiftItemLevels(groupIds, -1);

    if (lastChildId !== parent.id) {
      // Move after the last child
      const moved = await this.api.moveItems(groupIds, item.pageId, `after:${lastChildId}`);
      this.state.replaceItems(groupIds, moved, `after:${lastChildId}`);
      this.undo.begin('outdent');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
    } else {
      // The parent has no other child: the group only changes its level
      this.undo.begin('outdent');
      for (const b of beforeSnapshots) {
        this.undo.pushUpdate(b, { ...b, level: b.level - 1 });
      }
    }
    this.undo.commit();
  }

  /**
   * Changes the level of a group of items on screen, records the change for undo and saves it.
   * @param group - The items whose level changes
   * @param delta - The amount added to each level; negative for outdent
   * @param label - The undo label
   * @param levelOffset - Shown level minus stored level, for items inside a transclusion
   */
  private async shiftLevels(group: Item[], delta: number, label: string, levelOffset = 0): Promise<void> {
    this.undo.begin(label);
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + delta });
      this.state.updateItem(b.id, { level: b.level - levelOffset + delta });
    }
    this.undo.commit();
    await this.saveLevelShift(group, delta);
  }

  /**
   * Saves an indent or outdent that is already shown on screen.
   * The whole group goes in one relative request, and the response is not written back
   * to the state: a response to an older key press would undo a newer one.
   * If the save fails, the affected pages are reloaded so the screen matches the server again.
   * @param group - The items whose level changed
   * @param delta - The amount added to each level; negative for outdent
   */
  private async saveLevelShift(group: Item[], delta: number): Promise<void> {
    if (group.length === 0) return;
    try {
      await this.api.shiftItemLevels(group.map(b => b.id), delta);
    } catch (e) {
      console.error('Failed to change item levels:', e);
      for (const pageId of new Set(group.map(b => b.pageId))) {
        this.reloadPageFromFile(pageId);
      }
    }
  }

  // ---- Moving items ----

  /**
   * Handles the user pressing Alt+Shift+Up to move a item and its children up
   * past the previous sibling tree.
   * @param item - The item to move up
   */
  onMoveUp(item: Item): Promise<void> {
    return this.enqueueMove(() => this.moveUp(this.currentItem(item)));
  }

  /**
   * Handles the user pressing Alt+Shift+Down to move a item and its children down
   * past the next sibling tree.
   * @param item - The item to move down
   */
  onMoveDown(item: Item): Promise<void> {
    return this.enqueueMove(() => this.moveDown(this.currentItem(item)));
  }

  /**
   * Runs moves one after another. A move computes its target from the item list,
   * and the list changes only when the server has answered. A second key press that
   * did not wait would compute the same target again and the press would be lost.
   * @param move - The move to run when the moves before it are done
   * @returns A promise that settles when this move is done
   */
  private enqueueMove(move: () => Promise<void>): Promise<void> {
    const run = this.moveQueue.then(move);
    this.moveQueue = run.catch(() => undefined);
    return run;
  }

  /** The item as it is shown now; the one passed by the template may be from before an earlier move. */
  private currentItem(item: Item): Item {
    return this.state.visibleItems().find(b => b.id === item.id) ?? item;
  }

  private async moveUp(item: Item): Promise<void> {
    await this.edits.flushPendingEdit(item);

    const tCtx = this.getTransclusionContext(item);
    if (tCtx && !this.state.isOwnItem(item.id)) {
      await this.moveWithinTransclusion(item, tCtx, 'up');
      return;
    }

    if (this.isDailyJournalItem(item)) {
      const pageItems = this.state.items().filter(b => b.pageId === item.pageId);
      // The topmost root item moves to the end of the next day's page
      if (item.level === 1 && pageItems.find(b => b.level === 1)?.id === item.id) {
        await this.moveItemToAdjacentDay(item, 'next');
        return;
      }
      // A level 2 item at the second position moves to the next day's page
      if (item.level === 2 && pageItems.length >= 2 && pageItems[1].id === item.id) {
        const moved = await this.moveLevel2ToAdjacentDay(item, 'next');
        if (moved) return;
      }
    }

    const target = findMoveUpTarget(this.moveCandidates(item), item.id);
    if (!target) return;
    await this.moveGroupTo(item, target.position);
  }

  private async moveDown(item: Item): Promise<void> {
    await this.edits.flushPendingEdit(item);

    const tCtx = this.getTransclusionContext(item);
    if (tCtx && !this.state.isOwnItem(item.id)) {
      await this.moveWithinTransclusion(item, tCtx, 'down');
      return;
    }

    if (this.isDailyJournalItem(item)) {
      // The bottommost item moves to the previous day's page
      const pageItems = this.state.items().filter(b => b.pageId === item.pageId);
      if (pageItems.length > 0 && pageItems[pageItems.length - 1].id === item.id) {
        if (item.level === 1) {
          await this.moveItemToAdjacentDay(item, 'prev');
          return;
        } else if (item.level === 2) {
          const moved = await this.moveLevel2ToAdjacentDay(item, 'prev');
          if (moved) return;
        }
      }
    }

    const target = findMoveDownTarget(this.moveCandidates(item), item.id);
    if (!target) return;
    await this.moveGroupTo(item, target.position);
  }

  /**
   * The items a move target is searched in. In journal mode this is the item's own page,
   * so no position refers to another page, which the backend cannot resolve.
   */
  private moveCandidates(item: Item): Item[] {
    return this.isDailyJournalItem(item)
      ? this.state.items().filter(b => b.pageId === item.pageId)
      : this.state.items();
  }

  /** Moves an item with its children to a position on its own page, keeping its editor open. */
  private async moveGroupTo(item: Item, position: ItemPosition): Promise<void> {
    const statePosition = this.toFullListPosition(position, item.pageId);
    const group = this.state.getItemWithChildren(item.id);
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    const moved = await this.api.moveItems(groupIds, item.pageId, position);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    // Reordering the list blurs the editor; the edit goes on in the moved item
    await this.edits.handOverEdit(async () => {
      this.state.replaceItems(groupIds, moved, statePosition);
      await this.focusEditor(item.id);
    });
  }

  /** Moves a transcluded descendant within the tree of its transclusion root. */
  private async moveWithinTransclusion(item: Item, tCtx: TransclusionContext, direction: 'up' | 'down'): Promise<void> {
    const scopedItems = transclusionScopedItems(this.state.visibleItems(), tCtx);
    const target = direction === 'up'
      ? findMoveUpTarget(scopedItems, item.id)
      : findMoveDownTarget(scopedItems, item.id);
    if (!target) return;

    const groupIds = this.getVisibleSubtree(item.id).map(b => b.id);
    await this.api.moveItems(groupIds, item.pageId, target.position);
    // Reload to reflect reordering
    await this.edits.handOverEdit(async () => {
      await this.loadItems(this.pageId());
      await this.focusEditor(item.id);
    });
  }

  /** True for an item of a daily journal page (named YYYY-MM-DD). */
  private isDailyJournalItem(item: Item): boolean {
    return item.type === 'journal' && classifyJournalFileName(item.name) === 'daily';
  }

  /**
   * Translates a page-scoped position to a full-list position for state updates.
   * 'first' becomes the position before the first item of the page in the full list.
   * 'after:X' positions are already valid in the full list.
   */
  private toFullListPosition(position: ItemPosition, pageId: string): ItemPosition {
    if (position !== 'first') return position;
    const allItems = this.state.items();
    const firstPageIdx = allItems.findIndex(b => b.pageId === pageId);
    if (firstPageIdx <= 0) return 'first';
    return `after:${allItems[firstPageIdx - 1].id}`;
  }

  /** The date of the day before or after a daily journal item. */
  private adjacentDate(item: Item, direction: 'next' | 'prev'): string {
    return shiftIsoDate(item.name, direction === 'next' ? 1 : -1);
  }

  /** A page must keep at least one item: creates an empty one when the moved items were the last ones. */
  private async keepSourcePageNonEmpty(sourcePageId: string, movedIds: string[]): Promise<void> {
    const remaining = this.state.items().filter(b => b.pageId === sourcePageId && !movedIds.includes(b.id));
    if (remaining.length === 0) {
      await this.api.createItem(sourcePageId, 'first', 1, '');
    }
  }

  /**
   * Moves a journal item (and its children) to an adjacent day's page.
   * For 'next': moves to the next day's page at 'last' position.
   * For 'prev': moves to the previous day's page at 'first' position.
   * If the target page has only one item with empty body, that item is deleted after the move.
   */
  private async moveItemToAdjacentDay(item: Item, direction: 'next' | 'prev'): Promise<void> {
    const targetDate = this.adjacentDate(item, direction);

    // Get or create the target day's journal
    const result = await this.api.getOrCreateJournal(targetDate);
    const targetPageId = result.page.id;
    const targetItems = result.items;

    const groupIds = this.state.getItemWithChildren(item.id).map(b => b.id);

    const position: ItemPosition = direction === 'next' ? 'last' : 'first';
    await this.api.moveItems(groupIds, targetPageId, position);

    // If target page had only one item with empty body, delete it
    if (targetItems.length === 1 && targetItems[0].body === '') {
      await this.api.deleteItem(targetItems[0].id);
    }

    await this.keepSourcePageNonEmpty(item.pageId, groupIds);

    // Notify journal to reload, with navigation for move-up when URL matches
    const shouldNavigate = direction === 'next' && item.pageId === this.pageId();
    this.crossDayMoved.emit({ targetDate, navigate: shouldNavigate, itemId: item.id });
  }

  /**
   * Moves a level 2 journal item to an adjacent day's page, if the target page
   * exists and has a valid placement position.
   * For 'next': scans from the end of the next day's page.
   * For 'prev': scans from the start of the previous day's page.
   * @returns true if the item was moved, false otherwise
   */
  private async moveLevel2ToAdjacentDay(item: Item, direction: 'next' | 'prev'): Promise<boolean> {
    const targetDate = this.adjacentDate(item, direction);

    // Only proceed if target day's page is already loaded
    const targetPageItems = this.state.items().filter(b =>
      b.type === 'journal' && b.name === targetDate
    );
    if (targetPageItems.length === 0) return false;

    const scanDirection = direction === 'next' ? 'from-end' : 'from-start';
    const placement = findCrossPagePlacement(targetPageItems, item.level, scanDirection);
    if (!placement) return false;

    const groupIds = this.state.getItemWithChildren(item.id).map(b => b.id);
    const targetPageId = targetPageItems[0].pageId;

    await this.api.moveItems(groupIds, targetPageId, placement);
    await this.keepSourcePageNonEmpty(item.pageId, groupIds);

    this.crossDayMoved.emit({ targetDate, navigate: false, itemId: item.id });
    return true;
  }

  // ---- Wiki links ----

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

  // ---- Tasks, zoom, collapse, transclusion, archive ----

  /**
   * Handles the user clicking a task checkbox or pressing Ctrl+Enter.
   * Cycles through: normal → task → running → done → normal via cycleTask API.
   */
  async onTaskCycleRequested(item: Item): Promise<void> {
    try {
      // For transclusion items, target the source item
      const targetId = this.edits.saveTargetId(item);
      const originalItem = { ...item };
      const wasTask = item.types?.includes('task');
      let updated = await this.api.cycleTask(targetId);

      // When an item just became a task on a daily journal, set plan to the journal date
      if (!wasTask && updated.types?.includes('task') && this.isDailyJournalItem(item)) {
        updated = await this.api.upsertItemProperty(targetId, 'plan', 'date', item.name);
      }

      const change = { types: updated.types, properties: updated.properties };
      this.state.updateItem(targetId, change);
      this.undo.begin('cycle-task');
      this.undo.pushUpdate(originalItem, { ...originalItem, ...change });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to cycle task:', e);
    }
  }

  /**
   * Handles Shift+Wheel on an item to change its zoom level.
   */
  onItemWheel(item: Item, event: WheelEvent): void {
    if (!event.shiftKey) return;
    event.preventDefault();
    const current = parseFloat(item.properties['zoom'] as string) || 1;
    const delta = event.deltaY < 0 ? 0.1 : -0.1;
    const newZoom = Math.round(Math.min(3.0, Math.max(0.1, current + delta)) * 10) / 10;
    if (newZoom === current) return;
    this.applyZoom(item, newZoom);
  }

  /**
   * Resets zoom to 1.0 for the given item.
   */
  async onResetZoom(item: Item): Promise<void> {
    await this.applyZoom(item, 1.0);
  }

  private async applyZoom(item: Item, zoom: number): Promise<void> {
    const props = { ...item.properties, zoom: String(zoom) };
    this.state.updateItem(item.id, { properties: props });
    try {
      await this.api.upsertItemProperty(item.id, 'zoom', 'number', String(zoom));
    } catch (e: any) {
      console.error('Failed to update zoom:', e?.message || e);
    }
  }

  /**
   * Handles the user clicking the collapse/expand bullet or pressing the collapse shortcut.
   * Toggles the collapsed state locally for immediate feedback, then persists to the backend.
   * Reverts the state on error.
   * @param item - The item whose collapsed state was toggled
   */
  async onCollapseToggled(item: Item): Promise<void> {
    if (!this.state.hasChildren(item.id)) return;
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

  async onPasteAsTransclusion(item: Item): Promise<void> {
    const sourceId = (await navigator.clipboard.readText().catch(() => '')).trim();
    if (!sourceId || !/^[0-9A-Za-z_-]{10,}$/.test(sourceId)) return;
    try {
      const updated = await this.api.setTransclusion(item.id, sourceId);
      this.state.updateItem(item.id, updated);
      // Reload page to get transcluded trees
      await this.loadItems(this.pageId());
    } catch (e: any) {
      console.error('Failed to set transclusion:', e?.message || e);
    }
  }

  async onRemoveTransclusion(item: Item): Promise<void> {
    try {
      const updated = await this.api.removeTransclusion(item.id);
      this.state.updateItem(item.id, updated);
      // Reload page to refresh transcluded trees
      await this.loadItems(this.pageId());
    } catch (e) {
      console.error('Failed to remove transclusion:', e);
    }
  }

  /**
   * Sends an item (and its children) to the page's archive (#item-archive).
   * Appends #archived tag to the item body and moves it under the #item-archive item.
   * If no #item-archive item exists, creates one at the end of the page.
   */
  async onSendToArchive(item: Item): Promise<void> {
    try {
      const group = this.state.getItemWithChildren(item.id);
      if (group.length === 0) return;
      const groupIds = group.map(g => g.id);

      // Append #archived tag to the item body (skip if already present)
      if (!this.tagsOf(item).includes('archived')) {
        const newBody = item.body.trimEnd() + ' #archived';
        await this.edits.saveBody(item.id, newBody);
      }

      // Find or create the #item-archive item (identified by its tag).
      // Re-read items from state to pick up any updates from saveItemBody above
      let archiveItem = this.state.items().find(it => this.tagsOf(it).includes('item-archive'));
      if (!archiveItem) {
        // Create #item-archive at end of page
        const currentItems = this.state.items();
        const lastItem = currentItems[currentItems.length - 1];
        const created = await this.api.createItem(
          item.pageId,
          `after:${lastItem.id}`,
          1,
          '#item-archive',
        );
        // createItem API returns empty properties; patch tag so subsequent lookups work
        archiveItem = { ...created, properties: { ...created.properties, tag: ['item-archive'] } };
        this.state.addItem(archiveItem, `after:${lastItem.id}`);
      }

      // Move the archived items as the first child of #item-archive
      const levelDelta = (archiveItem.level + 1) - item.level;
      const moved = await this.api.moveItems(groupIds, item.pageId, `after:${archiveItem.id}`, levelDelta);
      this.state.replaceItems(groupIds, moved, `after:${archiveItem.id}`);
    } catch (e: any) {
      console.error('Failed to archive item:', e?.message || e);
    }
  }

  /** The hashtags of an item, as kept in its tag property. */
  private tagsOf(item: Item): string[] {
    const tags = item.properties['tag'];
    return Array.isArray(tags) ? tags : [];
  }

  // ---- Paste ----

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

    // Plain text pasted inside an open code fence stays in this item as it is
    const skipSplit = isPlainText && hasUnclosedCodeFence(item.body.substring(0, event.cursorPos));
    const parsedItems = skipSplit
      ? [{ body: textToParse, level: 0 }]
      : await this.splitTextIntoItems(textToParse);

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
      await this.edits.saveBody(item.id, firstItem.body);
      this.edits.markDirty(item.id);

      const afterFirstItem = this.state.getItemById(item.id);
      if (afterFirstItem) {
        this.undo.pushUpdate(originalItem, { ...afterFirstItem });
      }

      // Add remaining items after the current item
      let prevItemId = item.id;
      for (let i = 1; i < parsedItems.length; i++) {
        const parsedItem = parsedItems[i];
        const afterPos: ItemPosition = `after:${prevItemId}`;
        const newItem = await this.api.createItem(
          item.pageId,
          afterPos,
          item.level + parsedItem.level,
          parsedItem.body
        );
        this.state.addItems([newItem], afterPos);

        const finalNewItem = this.state.getItemById(newItem.id);
        this.undo.pushCreate(finalNewItem ?? newItem);

        prevItemId = newItem.id;
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

      await this.edits.saveBody(item.id, newBody);
      this.edits.markDirty(item.id);

      // Place cursor after the inserted markdown
      const newPos = event.cursorPos + markdown.length;
      this.state.setFocusedItem(item.id, newPos);
      this.focusEditor(item.id);
    } catch (e) {
      console.error('Failed to paste image:', e);
    }
  }

  /**
   * Splits text into items with the server-side Flat Markdown parser, so that pasted text
   * is cut at the same block boundaries as a markdown file. Levels are made relative to the
   * first item (0-based), because the caller places them under the current item.
   */
  private async splitTextIntoItems(text: string): Promise<Array<{ body: string; level: number }>> {
    const parsed = (await this.api.parseFlatMarkdown(text)).items;
    if (parsed.length === 0) return [];
    const topLevel = parsed[0].level;
    return parsed.map(p => ({ body: p.body, level: Math.max(0, p.level - topLevel) }));
  }

  // ---- Multi-item selection ----

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
    // Initial selection: only the current item, no subtree
    this.enterSelectionMode(item.id, new Set([item.id]));
  }

  /**
   * Handles the user dragging the mouse beyond the bounds of a item editor.
   * Exits edit mode and enters multi-item selection mode with drag tracking.
   * @param item - The item where the drag started
   * @param direction - Whether the drag went 'above' or 'below' the item
   */
  onDragBeyondBounds(item: Item, direction: 'above' | 'below'): void {
    this.state.removeAllEditingItems();
    this.state.setItemSelectionMode(true);
    this.selectionAnchorItemId = item.id;
    this.isSelectionDragging = true;

    // Select anchor item + children
    const group = this.state.getItemWithChildren(item.id);
    this.state.setSelectedItemIds(new Set(group.map(b => b.id)));

    // The click listener is added on mouse up, so the release does not end the selection
    this.listenForSelectionEvents('mouseup', 'keydown', 'copy', 'cut');
  }

  /**
   * Selects the given items with the keyboard listeners of selection mode active.
   * @param anchorItemId - The item the selection extends from
   * @param selectedIds - The items selected to start with
   */
  private enterSelectionMode(anchorItemId: string, selectedIds: Set<string>): void {
    this.state.setSelectedItemIds(selectedIds);
    this.state.setItemSelectionMode(true);
    this.selectionAnchorItemId = anchorItemId;
    this.selectionExtentItemId = anchorItemId;
    this.listenForSelectionEvents('keydown', 'click', 'copy', 'cut');
  }

  private exitSelectionMode(): void {
    this.state.clearSelection();
    this.selectionAnchorItemId = null;
    this.selectionExtentItemId = null;
    this.isSelectionDragging = false;
    this.stopListeningForSelectionEvents();
  }

  private listenForSelectionEvents(...names: SelectionEventName[]): void {
    for (const name of names) {
      document.addEventListener(name, this.selectionListeners[name] as EventListener);
    }
  }

  private stopListeningForSelectionEvents(): void {
    for (const name of Object.keys(this.selectionListeners) as SelectionEventName[]) {
      document.removeEventListener(name, this.selectionListeners[name] as EventListener);
    }
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

    const start = selectionStartIndex(items, anchorIdx, extentIdx);
    // When selecting upward into a subtree, the extent moves to the ancestor that was
    // pulled in, so the next Shift+Up continues from the correct position
    if (extentIdx < anchorIdx && start < extentIdx) {
      this.selectionExtentItemId = items[start].id;
    }
    this.selectRange(items, start, Math.max(anchorIdx, extentIdx));
  }

  /**
   * Handles the user's mouse entering a item during a drag selection.
   * Extends the selection range from the anchor item to the hovered item.
   * @param item - The item the mouse entered
   */
  onItemMouseEnter(item: Item): void {
    if (!this.isSelectionDragging) return;
    this.selectionExtentItemId = item.id;

    const items = this.state.visibleItems();
    const anchorIdx = items.findIndex(b => b.id === this.selectionAnchorItemId);
    const currentIdx = items.findIndex(b => b.id === item.id);
    if (anchorIdx < 0 || currentIdx < 0) return;

    this.selectRange(items, selectionStartIndex(items, anchorIdx, currentIdx), Math.max(anchorIdx, currentIdx));
  }

  /** Selects the items from start to end (inclusive) together with their subtrees. */
  private selectRange(items: Item[], start: number, end: number): void {
    this.state.setSelectedItemIds(idsOfRangeWithSubtrees(items, this.state.items(), start, end));
  }

  private onSelectionMouseUp(): void {
    this.isSelectionDragging = false;
    document.removeEventListener('mouseup', this.selectionListeners.mouseup);

    // Add click listener to detect clicks outside selection
    // Use setTimeout to avoid the current mouseup from triggering it
    setTimeout(() => this.listenForSelectionEvents('click'), 0);
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
        this.moveSelectedItems('up');
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.moveSelectedItems('down');
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

  private getOrderedSelectedItemIds(): string[] {
    const selectedIds = this.state.selectedItemIds();
    return this.state.visibleItems()
      .filter(b => selectedIds.has(b.id))
      .map(b => this.edits.saveTargetId(b));
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

    await this.ensureAtLeastOneItem();
  }

  private moveSelectedItems(direction: 'up' | 'down'): Promise<void> {
    return this.enqueueMove(() => this.moveSelection(direction));
  }

  /** Moves every selected subtree past the sibling tree above or below the selection. */
  private async moveSelection(direction: 'up' | 'down'): Promise<void> {
    const rootItems = selectedRootItems(this.state.items(), this.state.selectedItemIds());
    if (rootItems.length === 0) return;

    const edgeRoot = direction === 'up' ? rootItems[0] : rootItems[rootItems.length - 1];
    const target = direction === 'up'
      ? findMoveUpTarget(this.state.items(), edgeRoot.id)
      : findMoveDownTarget(this.state.items(), edgeRoot.id);
    if (!target) return;

    const allSelectedIds: string[] = [];
    for (const root of rootItems) {
      for (const b of this.state.getItemWithChildren(root.id)) {
        allSelectedIds.push(b.id);
      }
    }
    const beforeSnapshots = allSelectedIds
      .map(id => this.state.getItemById(id))
      .filter((b): b is Item => !!b)
      .map(b => ({ ...b }));

    try {
      const moved = await this.api.moveItems(allSelectedIds, edgeRoot.pageId, target.position);
      this.state.replaceItems(allSelectedIds, moved, target.position);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error(`Failed to move selected items ${direction}:`, e);
    }
  }

  private async indentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    const itemsToIndent = allItems.filter(b => selectedIds.has(b.id));

    if (itemsToIndent.length === 0) return;
    const firstIdx = allItems.findIndex(b => b.id === itemsToIndent[0].id);
    if (firstIdx <= 0) return;

    await this.shiftLevels(itemsToIndent, 1, 'indent-selection');
  }

  private async outdentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const itemsToOutdent = this.state.items().filter(b => selectedIds.has(b.id));

    if (itemsToOutdent.some(b => b.level <= 1)) return;

    await this.shiftLevels(itemsToOutdent, -1, 'outdent-selection');
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

  // ---- Undo and redo ----

  async performUndo(): Promise<void> {
    if (await this.undoReplay.replay('undo', this.undoEditor)) {
      await this.ensureAtLeastOneItem();
    }
  }

  async performRedo(): Promise<void> {
    if (await this.undoReplay.replay('redo', this.undoEditor)) {
      await this.ensureAtLeastOneItem();
    }
  }

  /** What a replay needs from the component: the edit in progress and the editor focus. */
  private readonly undoEditor = {
    discardPendingEdit: () => this.edits.discardPendingEditOfFocusedItem(),
    focusEditor: (itemId: string) => { this.focusEditor(itemId); },
  };

  // ---- Transclusion helpers ----

  private getTransclusionContext(item: Item): TransclusionContext | null {
    return transclusionContextOf(item, this.state.items(), this.state.transcludedTrees());
  }

  /** Gets an item and its subtree as shown, including transcluded descendants. */
  private getVisibleSubtree(itemId: string): Item[] {
    return getSubtree(this.state.visibleItems(), itemId);
  }
}
