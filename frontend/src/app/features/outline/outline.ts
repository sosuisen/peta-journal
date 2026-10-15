import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild, viewChildren, computed, untracked, OnDestroy } from '@angular/core';
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
import { formatJournalDate } from '../../core/utils/date.utils';
import { NavigationService } from '../../core/services/navigation.service';
import { WikiLinkContext, hasUnclosedCodeFence } from './item-editor';
import { ItemEditService } from './item-edit.service';
import { ItemMergeService } from './item-merge.service';
import { ItemMoveService, MoveDirection } from './item-move.service';
import { ItemSplitService } from './item-split.service';
import { CrossDayMove, isDailyJournalItem } from './journal-day-move.service';
import { OutlineSelection } from './outline-selection';
import { WikiSuggestionPick, WikiSuggestionsComponent } from './wiki-suggestions';
import { LevelShiftService } from './level-shift.service';
import { OutlineView } from './outline-view';
import { renderToHtml } from 'flatmarkdown-ast2html';
import { SseService } from '../../core/services/sse.service';


@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, ExperimentalScrollingModule, OutlineItemComponent, WikiSuggestionsComponent],
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
          [suggestionsVisible]="wiki.isShownFor(item.id)"
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
          (dragBeyondBounds)="onDragBeyondBounds(item)"
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
    <app-wiki-suggestions #wiki (picked)="onWikiSuggestionPicked($event)" />
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
  private readonly splitter = inject(ItemSplitService);
  private readonly merges = inject(ItemMergeService);
  private readonly moves = inject(ItemMoveService);
  private readonly levels = inject(LevelShiftService);
  private readonly sse = inject(SseService);
  private sseSub?: Subscription;
  private scrollSub?: Subscription;

  readonly pageId = input.required<string>();
  readonly focusItemId = input<string | null | undefined>('');
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic item loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  /** Emits after an item moved to another day's page. */
  readonly crossDayMoved = output<CrossDayMove>();
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

  private readonly wiki = viewChild.required(WikiSuggestionsComponent);

  // ---- Multi-item selection ----

  /** The selected items and the keys that act on them; see {@link OutlineSelection}. */
  private readonly selection = new OutlineSelection(this.state, {
    undo: () => { this.performUndo(); },
    redo: () => { this.performRedo(); },
    deleteSelected: () => { this.deleteSelectedItems(); },
    moveSelected: direction => { this.moveSelectedItems(direction); },
    indentSelected: () => { this.indentSelectedItems(); },
    outdentSelected: () => { this.outdentSelectedItems(); },
    copySelected: () => { this.copySelectedItems(); },
    cutSelected: () => { this.cutSelectedItems(); },
  });

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
    this.selection.dispose();
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
    this.selection.selectSubtree(itemId);
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
    this.wiki().close();
    await this.edits.finishEdit(item);
  }

  /** What the item services need from this component; see {@link OutlineView}. */
  private readonly view: OutlineView = {
    pageId: () => this.pageId(),
    reloadPage: () => this.loadItems(this.pageId()),
    reloadPageFromFile: pageId => { this.reloadPageFromFile(pageId); },
    focusEditor: itemId => this.focusEditor(itemId),
  };

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
    await this.splitter.handleEnter(item, event.cursorPos, this.view);
  }

  /**
   * Handles the user pressing Backspace at the start of a item.
   * Merges the current item's content into the previous item and deletes the current item.
   * @param item - The item where Backspace was pressed at position 0
   */
  async onBackspaceAtStart(item: Item): Promise<void> {
    await this.merges.mergeIntoPrevious(item, this.view);
  }

  /**
   * Handles the user pressing Delete at the end of a item.
   * Merges the next item's content into the current item and deletes the next item.
   * @param item - The item where Delete was pressed at the end
   */
  async onDeleteAtEnd(item: Item): Promise<void> {
    await this.merges.mergeNextInto(item);
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

  /** Handles the user pressing Tab to indent a item and its children by one level. */
  async onIndent(item: Item): Promise<void> {
    await this.levels.indent(item, this.view);
  }

  /** Handles the user pressing Shift+Tab to outdent an item and its children by one level. */
  async onOutdent(item: Item): Promise<void> {
    await this.levels.outdent(item, this.view);
  }

  // ---- Moving items ----

  /**
   * Handles the user pressing Alt+Shift+Up to move a item and its children up
   * past the previous sibling tree.
   * @param item - The item to move up
   */
  onMoveUp(item: Item): Promise<void> {
    return this.enqueueMove(() => this.moveItem(item, 'up'));
  }

  /**
   * Handles the user pressing Alt+Shift+Down to move a item and its children down
   * past the next sibling tree.
   * @param item - The item to move down
   */
  onMoveDown(item: Item): Promise<void> {
    return this.enqueueMove(() => this.moveItem(item, 'down'));
  }

  /** Moves the item as it is shown now and tells the journal when it changed day. */
  private async moveItem(item: Item, direction: MoveDirection): Promise<void> {
    const crossed = await this.moves.move(this.currentItem(item), direction, this.view);
    if (crossed) this.crossDayMoved.emit(crossed);
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

  // ---- Wiki links ----

  /**
   * Handles the user typing inside a `[[wiki link]]` in the item editor.
   * @param item - The item being edited
   * @param ctx - The wiki link context with query text and caret position, or null to close the popup
   */
  onWikiLinkQuery(item: Item, ctx: WikiLinkContext | null): void {
    this.wiki().query(item, ctx);
  }

  /**
   * Handles the user pressing arrow keys or Enter while the wiki link suggestion popup is open.
   * @param key - The key pressed ('ArrowUp', 'ArrowDown', or 'Enter')
   */
  onWikiLinkKeyNav(key: string): void {
    this.wiki().keyNav(key);
  }

  /**
   * Handles the user clicking on a wiki link to navigate to the linked page.
   * @param linkText - The text content of the wiki link (page title)
   */
  onOpenWikiLink(linkText: string): void {
    this.nav.navigateToPage(linkText);
  }

  /** Inserts the page title the user picked from the popup into the editor of the item. */
  onWikiSuggestionPicked(pick: WikiSuggestionPick): void {
    const itemComp = this.outlineItems()?.find(ob => ob.item().id === pick.itemId);
    itemComp?.insertWikiLinkText(pick.title);
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
      if (!wasTask && updated.types?.includes('task') && isDailyJournalItem(item)) {
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
   * Handles the user pressing Shift+Up to begin a multi-item selection.
   * @param item - The item where Shift+Up was pressed
   */
  onShiftSelectUp(item: Item): void {
    this.selection.startFromEditor(item);
  }

  /**
   * Handles the user pressing Shift+Down to begin a multi-item selection.
   * @param item - The item where Shift+Down was pressed
   */
  onShiftSelectDown(item: Item): void {
    this.selection.startFromEditor(item);
  }

  /**
   * Handles the user dragging the mouse beyond the bounds of a item editor.
   * Exits edit mode and enters multi-item selection mode with drag tracking.
   * @param item - The item where the drag started
   */
  onDragBeyondBounds(item: Item): void {
    this.selection.startDrag(item);
  }

  /**
   * Handles the user's mouse entering a item during a drag selection.
   * @param item - The item the mouse entered
   */
  onItemMouseEnter(item: Item): void {
    this.selection.extendDragTo(item);
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

    this.selection.exit();

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

  private moveSelectedItems(direction: MoveDirection): Promise<void> {
    return this.enqueueMove(() => this.moves.moveSelection(direction));
  }

  private async indentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const allItems = this.state.items();
    const itemsToIndent = allItems.filter(b => selectedIds.has(b.id));

    if (itemsToIndent.length === 0) return;
    const firstIdx = allItems.findIndex(b => b.id === itemsToIndent[0].id);
    if (firstIdx <= 0) return;

    await this.levels.shiftLevels(itemsToIndent, 1, 'indent-selection', this.view);
  }

  private async outdentSelectedItems(): Promise<void> {
    const selectedIds = this.state.selectedItemIds();
    const itemsToOutdent = this.state.items().filter(b => selectedIds.has(b.id));

    if (itemsToOutdent.some(b => b.level <= 1)) return;

    await this.levels.shiftLevels(itemsToOutdent, -1, 'outdent-selection', this.view);
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
}
