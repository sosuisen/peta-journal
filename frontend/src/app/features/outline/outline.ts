import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild, viewChildren, signal, OnDestroy } from '@angular/core';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { OutlineBlockComponent } from './outline-block';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { Block } from '../../core/models/block.model';
import { ClipboardService } from '../../core/services/clipboard.service';
import { UndoService, UndoEntry } from '../../core/services/undo.service';
import { formatJournalDate } from '../../core/utils/date.utils';
import { NavigationService } from '../../core/services/navigation.service';
import { WikiLinkContext } from './block-editor';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, OutlineBlockComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport [itemSize]="40" [minBufferPx]="500" [maxBufferPx]="600" class="outline-viewport" (scrolledIndexChange)="onScrolledIndexChange($event)">
      <div
        *cdkVirtualFor="let block of state.visibleBlocks(); let i = index; trackBy: trackById"
        class="outline-item"
        (mouseenter)="onBlockMouseEnter(block)"
      >
        @if (isFirstBlockOfDate(i) && block.fileName !== currentPageDate() && block.type === 'journal') {
          <div class="date-header">
            {{ formatDateHeader(block.fileName) }}
          </div>
        }
        <app-outline-block
          #outlineBlock
          [block]="block"
          [suggestionsVisible]="wikiSuggestionsBlockId() === block.id && wikiSuggestions().length > 0"
          (bodyChanged)="onBodyChanged(block, $event)"
          (editRequested)="onEditRequested(block.id, $event)"
          (editFinished)="onEditFinished(block)"
          (enterPressed)="onEnterPressed(block, $event)"
          (backspaceAtStart)="onBackspaceAtStart(block)"
          (deleteAtEnd)="onDeleteAtEnd(block)"
          (navigateUp)="onNavigateUp(block, $event)"
          (navigateDown)="onNavigateDown(block, $event)"
          (indentRequested)="onIndent(block)"
          (outdentRequested)="onOutdent(block)"
          (moveUp)="onMoveUp(block)"
          (moveDown)="onMoveDown(block)"
          (taskToggled)="onTaskStatusToggled(block)"
          (statusCycleRequested)="onStatusCycleRequested(block)"
          (collapseToggled)="onCollapseToggled(block)"
          (pasteDetected)="onPaste(block, $event)"
          (imagePasteDetected)="onImagePaste(block, $event)"
          (dragBeyondBounds)="onDragBeyondBounds(block, $event)"
          (wikiLinkQueryChanged)="onWikiLinkQuery(block, $event)"
          (wikiLinkSelected)="onWikiLinkKeyNav($event)"
          (openWikiLink)="onOpenWikiLink($event)"
          (shiftSelectUp)="onShiftSelectUp(block)"
          (shiftSelectDown)="onShiftSelectDown(block)"
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
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic block loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  readonly topBlockChanged = output<string>();  // Emits fileName of top visible block
  private readonly dirtyBlockIds = new Set<string>();
  private switchingEditBlock = false;
  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');
  private readonly outlineBlocks = viewChildren<OutlineBlockComponent>('outlineBlock');

  // Wiki link suggestions state
  readonly wikiSuggestions = signal<string[]>([]);
  readonly wikiSuggestionIndex = signal(0);
  readonly wikiSuggestionsBlockId = signal<string | null>(null);
  readonly wikiSuggestionsPos = signal({ x: 0, y: 0 });
  private wikiSearchTimer: ReturnType<typeof setTimeout> | null = null;

  private selectionAnchorBlockId: string | null = null;
  private selectionExtentBlockId: string | null = null;
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
      if (id) this.loadBlocks(id);
    });

    // Clear undo/redo stacks on page change
    effect(() => {
      this.pageId();
      this.undo.clear();
    });
  }

  /**
   * Handles the virtual scroll viewport emitting a new scrolled index.
   * Emits the top visible block's file name and triggers loading more blocks when nearing the end.
   * @param index - The index of the first visible item in the virtual scroll viewport
   */
  onScrolledIndexChange(index: number): void {
    const blocks = this.state.visibleBlocks();
    const totalBlocks = blocks.length;

    // Emit the top visible block's fileName
    if (index >= 0 && index < totalBlocks) {
      const topBlock = blocks[index];
      this.topBlockChanged.emit(topBlock.fileName);
    }

    // Check if we need more buffer (100 blocks ahead)
    if (totalBlocks < index + 100 && totalBlocks > 0) {
      const lastBlock = blocks[totalBlocks - 1];
      const filepath = lastBlock.dirPath + lastBlock.fileName;
      this.loadMore.emit({ filepath, currentIndex: index });
    }
  }

  /**
   * TrackBy function for the virtual scroll's cdkVirtualFor directive.
   * @param _index - The index of the item (unused)
   * @param block - The block to track
   * @returns The block's unique identifier
   */
  trackById(_index: number, block: Block): string {
    return block.id;
  }

  /**
   * Determines whether the block at the given index is the first block of a new date group.
   * Used in the template to render date header separators between journal entries.
   * @param index - The index of the block in the visible blocks array
   * @returns True if this block starts a new date group
   */
  isFirstBlockOfDate(index: number): boolean {
    const blocks = this.state.visibleBlocks();
    if (index === 0) return true;
    const currentBlock = blocks[index];
    const previousBlock = blocks[index - 1];
    return currentBlock.fileName !== previousBlock.fileName;
  }

  /**
   * Formats a journal file name into a human-readable date string for display as a date header.
   * @param fileName - The journal file name to format
   * @returns A formatted date string
   */
  formatDateHeader(fileName: string): string {
    return formatJournalDate(fileName);
  }

  private async loadBlocks(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPageById(pageId);
      this.state.setBlocks(result.blocks);
    } catch (e) {
      console.error('Failed to load blocks for page:', pageId, e);
    }
  }

  /**
   * Handles the user clicking on a block to start editing it.
   * Ignored when block selection mode is active.
   * @param blockId - The ID of the block to edit
   * @param cursorPosition - The cursor position to place the caret at
   */
  async onEditRequested(blockId: string, cursorPosition: number): Promise<void> {
    if (this.state.blockSelectionMode()) return;
    const currentId = this.state.focusedBlockId();
    if (currentId && currentId !== blockId) {
      const current = this.state.getBlockById(currentId);
      if (current) {
        this.switchingEditBlock = true;
        await this.saveBlockIfDirty(current);
        this.switchingEditBlock = false;
      }
    }
    this.state.addEditingBlock(blockId);
    this.state.setFocusedBlock(blockId, cursorPosition);
  }

  /**
   * Handles the user typing in a block editor, updating local state and scheduling an auto-save.
   * @param block - The block being edited
   * @param newBody - The updated body text
   */
  onBodyChanged(block: Block, newBody: string): void {
    const before = { ...block };
    this.state.updateBlock(block.id, { body: newBody });
    this.dirtyBlockIds.add(block.id);
    this.autoSave.scheduleBlockUpdate(block.id, newBody);
    const after = { ...block, body: newBody };
    this.undo.begin('body');
    this.undo.pushUpdate(before, after);
    this.undo.commit();
  }

  /**
   * Handles the user leaving a block editor (blur). Flushes pending changes to the backend
   * and switches the block back to view mode with updated HTML.
   * @param block - The block that was being edited
   */
  async onEditFinished(block: Block): Promise<void> {
    this.closeWikiSuggestions();
    if (this.switchingEditBlock) return;  // onEditRequested is handling save + switch

    // If not in editing set, another handler already removed it
    if (!this.state.editingBlockIds().has(block.id)) return;

    // If not focused, Enter handler is managing this block — don't interfere
    const isFocused = this.state.focusedBlockId() === block.id;
    if (!isFocused) return;

    if (!this.dirtyBlockIds.has(block.id)) {
      this.state.removeEditingBlock(block.id);
      return;
    }
    // Save dirty block
    this.dirtyBlockIds.delete(block.id);
    await this.autoSave.flushBlock(block.id);
    const current = this.state.getBlockById(block.id);
    if (current) {
      const updated = await this.api.updateBlock(block.id, { body: current.body });
      this.state.updateBlock(block.id, { html: updated.html });
    }
    this.state.removeEditingBlock(block.id);
  }

  /**
   * Handles the user pressing Enter in a block editor. Splits the block at the cursor position,
   * creating a new block with the text after the cursor. If the block is empty and indented,
   * outdents instead. If the cursor is at the beginning, inserts an empty block before.
   * @param block - The block where Enter was pressed
   * @param event - Contains the cursor position at the time of the keypress
   */
  async onEnterPressed(block: Block, event: { cursorPos: number }): Promise<void> {
    this.autoSave.cancelBlock(block.id);
    this.dirtyBlockIds.delete(block.id);

    const { cursorPos } = event;
    const beforeText = block.body.substring(0, cursorPos);
    const afterText = block.body.substring(cursorPos);

    if (block.body.length === 0 && block.level > 1) {
      // Check if there are same-level siblings below within the same parent
      const vBlocks = this.state.visibleBlocks();
      const vIdx = vBlocks.findIndex(b => b.id === block.id);
      let hasSiblingBelow = false;
      for (let i = vIdx + 1; i < vBlocks.length; i++) {
        if (vBlocks[i].level < block.level) break;
        if (vBlocks[i].level === block.level) { hasSiblingBelow = true; break; }
      }
      if (!hasSiblingBelow) {
        // No siblings below: outdent instead of creating new block
        await this.onOutdent(block);
        this.focusEditor(block.id);
        return;
      }
      // Has siblings below: fall through to create new block
    }

    if (cursorPos === 0 && block.body.length > 0) {
      // Enter at beginning: create new empty block before current block
      const allBlocks = this.state.blocks();
      const idx = allBlocks.findIndex(b => b.id === block.id);
      const prevBlockId = idx > 0 ? allBlocks[idx - 1].id : '';

      const newBlock = await this.api.createBlock(block.pageId, prevBlockId, block.level, '');
      this.state.addBlock(newBlock, prevBlockId);

      this.undo.begin('enter', { blockId: block.id, position: cursorPos });
      this.undo.pushCreate(newBlock);
      this.undo.commit({ blockId: newBlock.id, position: 0 });

      this.state.addEditingBlock(newBlock.id);
      this.focusEditor(newBlock.id);
    } else if (cursorPos === block.body.length) {
      // Enter at end: body unchanged — add optimistic block with client-generated ID
      const newId = crypto.randomUUID();
      const optimisticBlock: Block = {
        id: newId,
        pageId: block.pageId,
        dirPath: block.dirPath,
        fileName: block.fileName,
        type: block.type,
        createdDate: Date.now(),
        modifiedDate: Date.now(),
        idx: '',
        level: block.level,
        body: '',
        html: '',
        collapsed: false,
        status: 'normal',
        types: [],
        propertiesJson: '',
      };
      this.api.createBlock(block.pageId, block.id, block.level, '', newId);
      this.state.addBlock(optimisticBlock, block.id);

      this.undo.begin('enter', { blockId: block.id, position: cursorPos });
      this.undo.pushUpdate({ ...block }, { ...block });
      this.undo.pushCreate(optimisticBlock);
      this.undo.commit({ blockId: newId, position: 0 });

      this.dirtyBlockIds.delete(block.id);
      this.autoSave.cancelBlock(block.id);
      this.state.addEditingBlock(newId);
      this.state.setFocusedBlock(newId);
      this.focusEditor(newId);

      // Update old block body locally, keep editor visible during API calls
      this.state.updateBlock(block.id, { body: beforeText });
      const updatedOriginal = await this.api.updateBlock(block.id, { body: beforeText });
      // Update HTML before switching — prevents blink
      this.state.updateBlock(block.id, { html: updatedOriginal.html });
      this.state.removeEditingBlock(block.id);

    } else {
      const originalBlock = { ...block };

      // Update old block body locally, keep editor visible during API calls
      this.state.updateBlock(block.id, { body: beforeText });
      const [updatedOriginal, newBlock] = await Promise.all([
        this.api.updateBlock(block.id, { body: beforeText }),
        this.api.createBlock(block.pageId, block.id, block.level, afterText),
      ]);
      this.state.addBlock(newBlock, block.id);

      this.undo.begin('enter', { blockId: block.id, position: cursorPos });
      this.undo.pushUpdate(originalBlock, { ...updatedOriginal, body: beforeText });
      this.undo.pushCreate(newBlock);
      this.undo.commit({ blockId: newBlock.id, position: 0 });

      this.state.addEditingBlock(newBlock.id);
      this.state.setFocusedBlock(newBlock.id);
      this.focusEditor(newBlock.id);

      // Update HTML before switching — prevents blink
      this.state.updateBlock(block.id, { html: updatedOriginal.html });
      this.state.removeEditingBlock(block.id);
    }
  }

  /**
   * Handles the user pressing Backspace at the start of a block.
   * Merges the current block's content into the previous block and deletes the current block.
   * @param block - The block where Backspace was pressed at position 0
   */
  async onBackspaceAtStart(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;

    this.autoSave.cancelBlock(block.id);
    this.dirtyBlockIds.delete(block.id);

    const prevBlock = blocks[idx - 1];
    const originalPrev = { ...prevBlock };
    const originalBlock = { ...block };
    const cursorPos = prevBlock.body.length;
    const mergedBody = prevBlock.body + block.body;
    this.state.updateBlock(prevBlock.id, { body: mergedBody });
    this.state.deleteBlock(block.id);
    this.state.addEditingBlock(prevBlock.id);
    this.focusEditor(prevBlock.id);

    const updatedPrev = await this.api.updateBlock(prevBlock.id, { body: mergedBody });
    this.state.updateBlock(prevBlock.id, { html: updatedPrev.html });
    await this.api.deleteBlock(block.id);

    this.undo.begin('backspace', { blockId: block.id, position: 0 });
    this.undo.pushUpdate(originalPrev, { ...updatedPrev, body: mergedBody });
    this.undo.pushDelete(originalBlock);
    this.undo.commit({ blockId: prevBlock.id, position: cursorPos });
  }

  /**
   * Handles the user pressing Delete at the end of a block.
   * Merges the next block's content into the current block and deletes the next block.
   * @param block - The block where Delete was pressed at the end
   */
  async onDeleteAtEnd(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx >= blocks.length - 1) return;

    const nextBlock = blocks[idx + 1];
    this.autoSave.cancelBlock(nextBlock.id);
    this.dirtyBlockIds.delete(nextBlock.id);

    const originalBlock = { ...block };
    const originalNext = { ...nextBlock };
    const cursorPos = block.body.length;
    const mergedBody = block.body + nextBlock.body;
    this.state.updateBlock(block.id, { body: mergedBody });
    this.state.deleteBlock(nextBlock.id);

    const updatedBlock = await this.api.updateBlock(block.id, { body: mergedBody });
    this.state.updateBlock(block.id, { html: updatedBlock.html });
    await this.api.deleteBlock(nextBlock.id);

    this.undo.begin('delete', { blockId: block.id, position: cursorPos });
    this.undo.pushUpdate(originalBlock, { ...updatedBlock, body: mergedBody });
    this.undo.pushDelete(originalNext);
    this.undo.commit({ blockId: block.id, position: cursorPos });
  }

  /**
   * Handles the user pressing the Up arrow key to navigate to the previous block.
   * Saves any pending changes before moving focus.
   * @param block - The block where the Up arrow was pressed
   * @param cursorPos - The cursor position to restore in the target block
   */
  async onNavigateUp(block: Block, cursorPos: number): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx > 0) {
      // Save and update HTML before navigating
      await this.saveBlockIfDirty(block);
      this.state.addEditingBlock(blocks[idx - 1].id);
      this.state.setFocusedBlock(blocks[idx - 1].id, cursorPos);
      this.state.removeEditingBlock(block.id);
    }
  }

  /**
   * Handles the user pressing the Down arrow key to navigate to the next block.
   * Saves any pending changes before moving focus.
   * @param block - The block where the Down arrow was pressed
   * @param cursorPos - The cursor position to restore in the target block
   */
  async onNavigateDown(block: Block, cursorPos: number): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx < blocks.length - 1) {
      // Save and update HTML before navigating
      await this.saveBlockIfDirty(block);
      this.state.addEditingBlock(blocks[idx + 1].id);
      this.state.setFocusedBlock(blocks[idx + 1].id, cursorPos);
      this.state.removeEditingBlock(block.id);
    }
  }

  private async saveBlockIfDirty(block: Block): Promise<void> {
    if (!this.dirtyBlockIds.has(block.id)) return;

    this.dirtyBlockIds.delete(block.id);
    await this.autoSave.flushBlock(block.id);
    const current = this.state.getBlockById(block.id);
    if (current) {
      const updated = await this.api.updateBlock(block.id, { body: current.body });
      this.state.updateBlock(block.id, { html: updated.html });
    }
  }

  /**
   * Check if a block at the given level can be placed after prevBlock.
   * Constraint: the previous block's level must be the same as, or exactly 1 level above (parent).
   * i.e. prevLevel === blockLevel (sibling) or prevLevel === blockLevel - 1 (parent)
   * If no previous block, the block must be at level 0 (root).
   */
  private canPlace(blockLevel: number, prevBlock: Block | undefined): boolean {
    if (!prevBlock) return blockLevel === 1;
    return prevBlock.level === blockLevel || prevBlock.level === blockLevel - 1;
  }

  /**
   * Handles the user pressing Tab to indent a block and its children by one level.
   * Validates placement constraints before allowing the indent.
   * @param block - The block to indent
   */
  async onIndent(block: Block): Promise<void> {
    const blocks = this.state.blocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;
    const prevBlock = blocks[idx - 1];
    const newLevel = block.level + 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevBlock)) {
      // Exception: indenting by 1 level is allowed if there is a sibling at the same level
      // within the same parent subtree (stop scanning at shallower levels)
      let hasSameLevelBefore = false;
      for (let i = idx - 1; i >= 0; i--) {
        if (blocks[i].level < block.level) break; // exited current subtree
        if (blocks[i].level === block.level) {
          hasSameLevelBefore = true;
          break;
        }
      }
      if (!hasSameLevelBefore) return;
    }

    const group = this.state.getBlockWithChildren(block.id);
    this.undo.begin('indent');
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + 1 });
      this.state.updateBlock(b.id, { level: b.level + 1 });
    }
    this.undo.commit();
    await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level + 1 })));
  }

  /**
   * Handles the user pressing Shift+Tab to outdent a block and its children by one level.
   * May reposition the block after its parent's last child if normal placement is not possible.
   * @param block - The block to outdent
   */
  async onOutdent(block: Block): Promise<void> {
    if (block.level <= 1) return;
    const blocks = this.state.blocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    const prevBlock = idx > 0 ? blocks[idx - 1] : undefined;
    const newLevel = block.level - 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevBlock)) {
      // Exception: outdenting by 1 level is allowed by placing after the last child of the current parent
      let currentParentIdx = -1;
      for (let i = idx - 1; i >= 0; i--) {
        if (blocks[i].level === block.level - 1) {
          currentParentIdx = i;
          break;
        }
      }

      if (currentParentIdx >= 0) {
        const currentParent = blocks[currentParentIdx];
        const allChildren = this.state.getChildren(currentParent.id);
        const group = this.state.getBlockWithChildren(block.id);
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
          this.state.updateBlock(b.id, { level: b.level - 1 });
        }
        await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level - 1 })));

        // Move after the last child (if it's not the parent itself)
        if (lastChildId !== currentParent.id) {
          const moved = await this.api.moveBlocks(groupIds, block.pageId, lastChildId);
          this.state.replaceBlocks(groupIds, moved, lastChildId);

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
    const group = this.state.getBlockWithChildren(block.id);
    this.undo.begin('outdent');
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level - 1 });
      this.state.updateBlock(b.id, { level: b.level - 1 });
    }
    this.undo.commit();
    await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level - 1 })));
  }

  /**
   * Handles the user pressing Alt+Shift+Up to move a block and its children up
   * past the previous sibling tree.
   * @param block - The block to move up
   */
  async onMoveUp(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;

    const group = this.state.getBlockWithChildren(block.id);
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    // Standard placement: move entire tree before the entire previous tree
    const prevBlock = blocks[idx - 1];
    if (this.canPlace(block.level, prevBlock)) {
      const allBlocks = this.state.blocks();
      const prevBlockIdx = allBlocks.findIndex(b => b.id === prevBlock.id);
      const targetPrev = prevBlockIdx > 0 ? allBlocks[prevBlockIdx - 1] : undefined;
      const prevPrevId = targetPrev?.id ?? '';

      const moved = await this.api.moveBlocks(groupIds, block.pageId, prevPrevId);
      this.state.replaceBlocks(groupIds, moved, prevPrevId || '');

      this.undo.begin('move');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();

      this.focusEditor(block.id);
      return;
    }

    // Exception: If there's a block at the same level above, move just above it
    let sameLevelBlockIdx = -1;
    for (let i = idx - 1; i >= 0; i--) {
      if (blocks[i].level === block.level) {
        sameLevelBlockIdx = i;
        break;
      }
    }

    if (sameLevelBlockIdx === -1) {
      return;
    }

    const allBlocks = this.state.blocks();
    const sameLevelBlock = blocks[sameLevelBlockIdx];
    const sameLevelIdx = allBlocks.findIndex(b => b.id === sameLevelBlock.id);

    const targetPrev = sameLevelIdx > 0 ? allBlocks[sameLevelIdx - 1] : undefined;
    const targetPrevId = targetPrev?.id ?? '';

    const moved = await this.api.moveBlocks(groupIds, block.pageId, targetPrevId);
    this.state.replaceBlocks(groupIds, moved, targetPrevId);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    this.focusEditor(block.id);
  }

  /**
   * Handles the user pressing Alt+Shift+Down to move a block and its children down
   * past the next sibling tree.
   * @param block - The block to move down
   */
  async onMoveDown(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx >= blocks.length - 1) return;

    const children = this.state.getChildren(block.id);
    const group = [block, ...children];
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    const allBlocks = this.state.blocks();
    const lastChildIdx = allBlocks.findIndex(b => b.id === group[group.length - 1].id);
    if (lastChildIdx >= allBlocks.length - 1) return;
    const targetBlock = allBlocks[lastChildIdx + 1];

    // Standard placement: move entire tree after the entire next tree
    if (this.canPlace(block.level, targetBlock)) {
      const targetGroup = this.state.getBlockWithChildren(targetBlock.id);
      const lastTargetDescendant = targetGroup[targetGroup.length - 1];

      const moved = await this.api.moveBlocks(groupIds, block.pageId, lastTargetDescendant.id);
      this.state.replaceBlocks(groupIds, moved, lastTargetDescendant.id);

      this.undo.begin('move');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();

      this.focusEditor(block.id);
      return;
    }

    // Exception: If there's a block at the same level below, move just after it
    let sameLevelBlockIdx = -1;
    for (let i = idx + 1; i < blocks.length; i++) {
      if (blocks[i].level === block.level) {
        sameLevelBlockIdx = i;
        break;
      }
    }

    if (sameLevelBlockIdx === -1) {
      return;
    }

    const sameLevelBlock = blocks[sameLevelBlockIdx];
    const sameLevelGroup = this.state.getBlockWithChildren(sameLevelBlock.id);
    const lastDescendant = sameLevelGroup[sameLevelGroup.length - 1];

    const moved = await this.api.moveBlocks(groupIds, block.pageId, lastDescendant.id);
    this.state.replaceBlocks(groupIds, moved, lastDescendant.id);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    this.focusEditor(block.id);
  }

  /**
   * Handles the user typing inside a `[[wiki link]]` in the block editor.
   * Positions the suggestion popup and performs a debounced search for matching page names.
   * @param block - The block being edited
   * @param ctx - The wiki link context with query text and caret position, or null to close suggestions
   */
  onWikiLinkQuery(block: Block, ctx: WikiLinkContext | null): void {
    if (!ctx) {
      this.closeWikiSuggestions();
      return;
    }
    this.wikiSuggestionsBlockId.set(block.id);
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
   * Handles the user pressing Shift+Up to begin or extend a multi-block selection upward.
   * @param block - The block where Shift+Up was pressed
   */
  onShiftSelectUp(block: Block): void {
    this.startShiftSelection(block);
  }

  /**
   * Handles the user pressing Shift+Down to begin or extend a multi-block selection downward.
   * @param block - The block where Shift+Down was pressed
   */
  onShiftSelectDown(block: Block): void {
    this.startShiftSelection(block);
  }

  private startShiftSelection(block: Block): void {
    this.state.removeAllEditingBlocks();
    this.state.setBlockSelectionMode(true);
    this.selectionAnchorBlockId = block.id;
    this.selectionExtentBlockId = block.id;
    // Initial selection: only the current block, no subtree
    this.state.setSelectedBlockIds(new Set([block.id]));

    document.addEventListener('keydown', this.selectionKeydownHandler);
    document.addEventListener('click', this.selectionClickHandler);
    document.addEventListener('copy', this.selectionCopyHandler);
    document.addEventListener('cut', this.selectionCutHandler);
  }

  private shiftExtendUp(): void {
    const blocks = this.state.visibleBlocks();
    let extentIdx = blocks.findIndex(b => b.id === this.selectionExtentBlockId);
    const anchorIdx = blocks.findIndex(b => b.id === this.selectionAnchorBlockId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx <= 0) return;

    // When expanding upward, skip past blocks already in selection
    if (extentIdx <= anchorIdx) {
      const selected = this.state.selectedBlockIds();
      while (extentIdx - 1 >= 0 && selected.has(blocks[extentIdx - 1].id)) {
        extentIdx--;
      }
      if (extentIdx <= 0) return;
    }

    this.selectionExtentBlockId = blocks[extentIdx - 1].id;
    this.updateShiftSelection();
  }

  private shiftExtendDown(): void {
    const blocks = this.state.visibleBlocks();
    let extentIdx = blocks.findIndex(b => b.id === this.selectionExtentBlockId);
    const anchorIdx = blocks.findIndex(b => b.id === this.selectionAnchorBlockId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx >= blocks.length - 1) return;

    // When expanding downward, skip past blocks already in selection
    if (extentIdx >= anchorIdx) {
      const selected = this.state.selectedBlockIds();
      while (extentIdx + 1 < blocks.length && selected.has(blocks[extentIdx + 1].id)) {
        extentIdx++;
      }
      if (extentIdx >= blocks.length - 1) return;
    }

    this.selectionExtentBlockId = blocks[extentIdx + 1].id;
    this.updateShiftSelection();
  }

  private updateShiftSelection(): void {
    const blocks = this.state.visibleBlocks();
    const anchorIdx = blocks.findIndex(b => b.id === this.selectionAnchorBlockId);
    const extentIdx = blocks.findIndex(b => b.id === this.selectionExtentBlockId);
    if (anchorIdx < 0 || extentIdx < 0) return;

    const start = Math.min(anchorIdx, extentIdx);
    const end = Math.max(anchorIdx, extentIdx);
    const selectedSet = new Set<string>();
    for (let i = start; i <= end; i++) {
      const group = this.state.getBlockWithChildren(blocks[i].id);
      group.forEach(b => selectedSet.add(b.id));
    }
    this.state.setSelectedBlockIds(selectedSet);
  }

  /**
   * Handles the user selecting a wiki link suggestion by clicking on it or pressing Enter.
   * Inserts the selected page title into the block editor and closes the suggestions popup.
   * @param title - The page title selected from the suggestions
   * @param event - The mouse event if selected by click (used to prevent default behavior)
   */
  onWikiSuggestionSelected(title: string, event?: MouseEvent): void {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    const blockId = this.wikiSuggestionsBlockId();
    if (!blockId) return;
    const blockComp = this.outlineBlocks()?.find(
      ob => ob.block().id === blockId
    );
    blockComp?.insertWikiLinkText(title);
    this.closeWikiSuggestions();
  }

  private closeWikiSuggestions(): void {
    this.wikiSuggestions.set([]);
    this.wikiSuggestionsBlockId.set(null);
    if (this.wikiSearchTimer) {
      clearTimeout(this.wikiSearchTimer);
      this.wikiSearchTimer = null;
    }
  }

  /**
   * Handles the user clicking a task checkbox to cycle through statuses:
   * normal -> todo -> running -> done -> normal.
   * @param block - The block whose task status was toggled
   */
  async onTaskStatusToggled(block: Block): Promise<void> {
    try {
      const originalBlock = { ...block };
      const cycle = ['normal', 'todo', 'running', 'done'];
      const currentIndex = cycle.indexOf(block.status);
      const newStatus = cycle[(currentIndex + 1) % cycle.length];
      if (newStatus !== block.status) {
        const updated = await this.api.updateBlockStatus(block.id, newStatus);
        this.state.updateBlock(block.id, { status: updated.status });

        this.undo.begin('status');
        this.undo.pushUpdate(originalBlock, { ...originalBlock, status: newStatus });
        this.undo.commit();
      }
    } catch (e) {
      console.error('Failed to toggle task status:', e);
    }
  }

  /**
   * Handles the user pressing Ctrl+Enter to cycle a block's status:
   * normal -> todo -> running -> done -> normal.
   * @param block - The block whose status should be cycled
   */
  async onStatusCycleRequested(block: Block): Promise<void> {
    try {
      const originalBlock = { ...block };
      const cycle = ['normal', 'todo', 'running', 'done'];
      const currentIndex = cycle.indexOf(block.status);
      const newStatus = cycle[(currentIndex + 1) % cycle.length];
      const updated = await this.api.updateBlockStatus(block.id, newStatus);
      this.state.updateBlock(block.id, { status: updated.status });

      this.undo.begin('status');
      this.undo.pushUpdate(originalBlock, { ...originalBlock, status: newStatus });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to cycle status:', e);
    }
  }

  /**
   * Handles the user clicking the collapse/expand bullet or pressing the collapse shortcut.
   * Toggles the collapsed state locally for immediate feedback, then persists to the backend.
   * Reverts the state on error.
   * @param block - The block whose collapsed state was toggled
   */
  async onCollapseToggled(block: Block): Promise<void> {
    try {
      const originalBlock = { ...block };
      const newCollapsed = !block.collapsed;
      // Update local state immediately for responsive UI
      this.state.toggleCollapsed(block.id);
      this.state.updateBlock(block.id, { collapsed: newCollapsed });
      // Persist to backend
      await this.api.updateBlock(block.id, { collapsed: newCollapsed });

      this.undo.begin('collapse');
      this.undo.pushUpdate(originalBlock, { ...originalBlock, collapsed: newCollapsed });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to toggle collapse:', e);
      // Revert on error
      this.state.toggleCollapsed(block.id);
      this.state.updateBlock(block.id, { collapsed: block.collapsed });
    }
  }

  /**
   * Handles the user pasting content into a block editor. Parses the pasted text into blocks,
   * splits the current block at the cursor, and inserts the parsed blocks.
   * Supports plain text, tree markdown, and internally copied flat markdown.
   * @param block - The block where the paste occurred
   * @param event - Contains the pasted text, HTML, and cursor position at the time of paste
   */
  async onPaste(block: Block, event: { text: string; html: string; cursorPos: number }): Promise<void> {
    let textToParse = event.text;

    // Check 1: If we have stored flat markdown for this clipboard content, use it
    const flatMd = this.clipboard.consume(event.text);
    if (flatMd !== null) {
      textToParse = flatMd;
    }
    // Check 2: If text looks like tree markdown (starts with "- "), convert it
    else if (event.text.trimStart().startsWith('- ')) {
      try {
        const result = await this.api.importTreeMarkdown(event.text);
        textToParse = result.flatMarkdown;
      } catch (e) {
        console.error('Failed to import tree markdown, using raw text:', e);
      }
    }

    const parsedBlocks = this.parseTextIntoBlocks(textToParse);

    if (parsedBlocks.length === 0) return;

    const originalBlock = { ...block };

    // Split existing text at caret position
    const before = block.body.substring(0, event.cursorPos);
    const after = block.body.substring(event.cursorPos);

    // Merge existing text around pasted content
    parsedBlocks[0].body = before + parsedBlocks[0].body;
    parsedBlocks[parsedBlocks.length - 1].body = parsedBlocks[parsedBlocks.length - 1].body + after;

    try {
      this.undo.begin('paste', { blockId: block.id, position: event.cursorPos });

      // First block replaces current editing block
      const firstBlock = parsedBlocks[0];
      const updated = await this.api.updateBlock(block.id, { body: firstBlock.body });
      this.state.updateBlock(block.id, { body: firstBlock.body, html: updated.html });
      this.dirtyBlockIds.add(block.id);

      // Update status if needed
      if (firstBlock.status !== 'normal') {
        await this.api.updateBlockStatus(block.id, firstBlock.status);
        this.state.updateBlock(block.id, { status: firstBlock.status });
      }

      const afterFirstBlock = this.state.getBlockById(block.id);
      if (afterFirstBlock) {
        this.undo.pushUpdate(originalBlock, { ...afterFirstBlock });
      }

      // Add remaining blocks after the current block
      if (parsedBlocks.length > 1) {
        let prevBlockId = block.id;
        for (let i = 1; i < parsedBlocks.length; i++) {
          const parsedBlock = parsedBlocks[i];
          const newBlock = await this.api.createBlock(
            block.pageId,
            prevBlockId,
            block.level + parsedBlock.level,
            parsedBlock.body
          );
          this.state.addBlocks([newBlock], prevBlockId);

          // Update status if needed
          if (parsedBlock.status !== 'normal') {
            await this.api.updateBlockStatus(newBlock.id, parsedBlock.status);
            this.state.updateBlock(newBlock.id, { status: parsedBlock.status });
          }

          const finalNewBlock = this.state.getBlockById(newBlock.id);
          this.undo.pushCreate(finalNewBlock ?? newBlock);

          prevBlockId = newBlock.id;
        }
      }

      this.undo.commit();

      // Focus the last created block or the current block
      this.focusEditor();
    } catch (e) {
      console.error('Failed to process paste:', e);
      this.undo.abort();
    }
  }

  async onImagePaste(block: Block, event: { file: File; cursorPos: number }): Promise<void> {
    try {
      const result = await this.api.uploadAsset(event.file);
      const fileName = result.fileName;
      const markdown = `![${fileName}](../assets/${fileName})`;

      const before = block.body.substring(0, event.cursorPos);
      const after = block.body.substring(event.cursorPos);
      const newBody = before + markdown + after;

      this.state.updateBlock(block.id, { body: newBody });
      this.dirtyBlockIds.add(block.id);

      const updated = await this.api.updateBlock(block.id, { body: newBody });
      this.state.updateBlock(block.id, { html: updated.html });

      // Place cursor after the inserted markdown
      const newPos = event.cursorPos + markdown.length;
      this.state.setFocusedBlock(block.id, newPos);
      this.focusEditor(block.id);
    } catch (e) {
      console.error('Failed to paste image:', e);
    }
  }

  private parseTextIntoBlocks(text: string): Array<{ body: string; status: string; level: number }> {
    const blocks: Array<{ body: string; status: string; level: number }> = [];
    const lines = text.split('\n');
    let inCodeFence = false;

    // Current block accumulation
    let currentBodyLines: string[] = [];
    let currentLevel = 1;
    let hasContent = false;

    const flushBlock = () => {
      if (!hasContent) return;
      let body = currentBodyLines.join('\n');
      let status = 'normal';

      // Detect status from first body line
      if (body.startsWith('- [ ] ')) {
        status = 'todo';
        body = body.substring(6);
      } else if (body.startsWith('- [x] ')) {
        status = 'done';
        body = body.substring(6);
      }

      // Whitespace-only body → empty
      if (body.length > 0 && body.trim() === '') body = '';
      blocks.push({ body, status, level: currentLevel });
      currentBodyLines = [];
      currentLevel = 1;
      hasContent = false;
    };

    for (const line of lines) {
      // Track code fences
      if (line.trim().startsWith('```')) {
        inCodeFence = !inCodeFence;
      }

      // Blank line outside code fence → flush current block
      if (line === '' && !inCodeFence) {
        flushBlock();
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

    // Flush last block
    flushBlock();

    // Normalize levels: subtract the top block's level to get relative offsets (0-based)
    if (blocks.length > 0) {
      const topLevel = blocks[0].level;
      for (const block of blocks) {
        block.level = Math.max(0, block.level - topLevel);
      }
    }

    return blocks;
  }

  /**
   * Handles the user dragging the mouse beyond the bounds of a block editor.
   * Exits edit mode and enters multi-block selection mode with drag tracking.
   * @param block - The block where the drag started
   * @param direction - Whether the drag went 'above' or 'below' the block
   */
  onDragBeyondBounds(block: Block, direction: 'above' | 'below'): void {
    // Exit edit mode
    this.state.removeAllEditingBlocks();
    // Enter selection mode
    this.state.setBlockSelectionMode(true);
    this.selectionAnchorBlockId = block.id;
    this.isSelectionDragging = true;

    // Select anchor block + children
    const group = this.state.getBlockWithChildren(block.id);
    const selectedSet = new Set<string>(group.map(b => b.id));
    this.state.setSelectedBlockIds(selectedSet);

    // Register document-level listeners
    document.addEventListener('mouseup', this.selectionMouseUpHandler);
    document.addEventListener('keydown', this.selectionKeydownHandler);
    document.addEventListener('copy', this.selectionCopyHandler);
    document.addEventListener('cut', this.selectionCutHandler);
  }

  /**
   * Handles the user's mouse entering a block during a drag selection.
   * Extends the selection range from the anchor block to the hovered block.
   * @param block - The block the mouse entered
   */
  onBlockMouseEnter(block: Block): void {
    if (!this.isSelectionDragging) return;
    this.selectionExtentBlockId = block.id;

    const visibleBlocks = this.state.visibleBlocks();
    const anchorIdx = visibleBlocks.findIndex(b => b.id === this.selectionAnchorBlockId);
    const currentIdx = visibleBlocks.findIndex(b => b.id === block.id);
    if (anchorIdx < 0 || currentIdx < 0) return;

    const [start, end] = [Math.min(anchorIdx, currentIdx), Math.max(anchorIdx, currentIdx)];
    const selectedSet = new Set<string>();
    for (let i = start; i <= end; i++) {
      // getBlockWithChildren uses _blocks() (all blocks) so collapsed children are included
      const group = this.state.getBlockWithChildren(visibleBlocks[i].id);
      group.forEach(b => selectedSet.add(b.id));
    }
    this.state.setSelectedBlockIds(selectedSet);
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
      this.deleteSelectedBlocks();
      return;
    }

    if (e.shiftKey && e.altKey) {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.moveSelectedBlocksUp();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.moveSelectedBlocksDown();
        return;
      }
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        this.outdentSelectedBlocks();
        return;
      }
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        this.indentSelectedBlocks();
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
        this.outdentSelectedBlocks();
      } else {
        this.indentSelectedBlocks();
      }
      return;
    }

    if (e.ctrlKey && e.key === 'c') {
      e.preventDefault();
      this.copySelectedBlocks();
      return;
    }

    if (e.ctrlKey && e.key === 'x') {
      e.preventDefault();
      this.cutSelectedBlocks();
      return;
    }
  }

  private getSelectedRootBlocks(): Block[] {
    const selectedIds = this.state.selectedBlockIds();
    const allBlocks = this.state.blocks();
    const roots: Block[] = [];

    for (const block of allBlocks) {
      if (!selectedIds.has(block.id)) continue;
      // Check if parent is in selection
      const idx = allBlocks.indexOf(block);
      let isRoot = true;
      for (let i = idx - 1; i >= 0; i--) {
        if (allBlocks[i].level < block.level) {
          if (selectedIds.has(allBlocks[i].id)) {
            isRoot = false;
          }
          break;
        }
      }
      if (isRoot) roots.push(block);
    }
    return roots;
  }

  private getOrderedSelectedBlockIds(): string[] {
    const selectedIds = this.state.selectedBlockIds();
    const allBlocks = this.state.blocks();
    return allBlocks.filter(b => selectedIds.has(b.id)).map(b => b.id);
  }

  private async deleteSelectedBlocks(): Promise<void> {
    const selectedIds = this.getOrderedSelectedBlockIds();
    if (selectedIds.length === 0) return;

    // Snapshot before deletion
    const deletedBlocks = selectedIds.map(id => this.state.getBlockById(id)).filter((b): b is Block => !!b);

    this.exitSelectionMode();

    this.undo.begin('delete-selection');
    for (const b of deletedBlocks) {
      this.undo.pushDelete({ ...b });
    }
    this.undo.commit();

    // Delete from state first for responsiveness
    this.state.deleteBlocks(selectedIds);

    // Delete from backend
    try {
      await Promise.all(selectedIds.map(id => this.api.deleteBlock(id)));
    } catch (e) {
      console.error('Failed to delete selected blocks:', e);
    }
  }

  private async moveSelectedBlocksUp(): Promise<void> {
    const rootBlocks = this.getSelectedRootBlocks();
    if (rootBlocks.length === 0) return;

    const minLevel = Math.min(...rootBlocks.map(b => b.level));
    const firstRoot = rootBlocks[0];
    const visibleBlocks = this.state.visibleBlocks();
    const firstIdx = visibleBlocks.findIndex(b => b.id === firstRoot.id);
    if (firstIdx <= 0) return;

    const prevBlock = visibleBlocks[firstIdx - 1];

    const allSelectedIds: string[] = [];
    for (const root of rootBlocks) {
      const group = this.state.getBlockWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }
    const beforeSnapshots = allSelectedIds.map(id => {
      const b = this.state.getBlockById(id);
      return b ? { ...b } : null;
    }).filter((b): b is Block => !!b);

    const allBlocks = this.state.blocks();

    if (this.canPlace(minLevel, prevBlock)) {
      const prevBlockAllIdx = allBlocks.findIndex(b => b.id === prevBlock.id);
      const targetPrev = prevBlockAllIdx > 0 ? allBlocks[prevBlockAllIdx - 1] : undefined;
      const targetPrevId = targetPrev?.id ?? '';

      try {
        const moved = await this.api.moveBlocks(allSelectedIds, firstRoot.pageId, targetPrevId);
        this.state.replaceBlocks(allSelectedIds, moved, targetPrevId);

        this.undo.begin('move-selection');
        for (let i = 0; i < beforeSnapshots.length; i++) {
          this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
        }
        this.undo.commit();
      } catch (e) {
        console.error('Failed to move selected blocks up:', e);
      }
      return;
    }

    let sameLevelBlockIdx = -1;
    for (let i = firstIdx - 1; i >= 0; i--) {
      if (visibleBlocks[i].level === minLevel) {
        sameLevelBlockIdx = i;
        break;
      }
    }
    if (sameLevelBlockIdx === -1) return;

    const sameLevelBlock = visibleBlocks[sameLevelBlockIdx];
    const sameLevelIdx = allBlocks.findIndex(b => b.id === sameLevelBlock.id);
    const targetPrev = sameLevelIdx > 0 ? allBlocks[sameLevelIdx - 1] : undefined;
    const targetPrevId = targetPrev?.id ?? '';

    try {
      const moved = await this.api.moveBlocks(allSelectedIds, firstRoot.pageId, targetPrevId);
      this.state.replaceBlocks(allSelectedIds, moved, targetPrevId);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error('Failed to move selected blocks up:', e);
    }
  }

  private async moveSelectedBlocksDown(): Promise<void> {
    const rootBlocks = this.getSelectedRootBlocks();
    if (rootBlocks.length === 0) return;

    const minLevel = Math.min(...rootBlocks.map(b => b.level));
    const lastRoot = rootBlocks[rootBlocks.length - 1];
    const lastRootGroup = this.state.getBlockWithChildren(lastRoot.id);
    const lastBlock = lastRootGroup[lastRootGroup.length - 1];

    const allBlocks = this.state.blocks();
    const lastBlockIdx = allBlocks.findIndex(b => b.id === lastBlock.id);
    if (lastBlockIdx >= allBlocks.length - 1) return;

    const nextBlock = allBlocks[lastBlockIdx + 1];

    const allSelectedIds: string[] = [];
    for (const root of rootBlocks) {
      const group = this.state.getBlockWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }
    const beforeSnapshots = allSelectedIds.map(id => {
      const b = this.state.getBlockById(id);
      return b ? { ...b } : null;
    }).filter((b): b is Block => !!b);

    if (this.canPlace(minLevel, nextBlock)) {
      const nextGroup = this.state.getBlockWithChildren(nextBlock.id);
      const targetAfter = nextGroup[nextGroup.length - 1];

      try {
        const moved = await this.api.moveBlocks(allSelectedIds, lastRoot.pageId, targetAfter.id);
        this.state.replaceBlocks(allSelectedIds, moved, targetAfter.id);

        this.undo.begin('move-selection');
        for (let i = 0; i < beforeSnapshots.length; i++) {
          this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
        }
        this.undo.commit();
      } catch (e) {
        console.error('Failed to move selected blocks down:', e);
      }
      return;
    }

    const visibleBlocks = this.state.visibleBlocks();
    const lastRootVisIdx = visibleBlocks.findIndex(b => b.id === lastRoot.id);
    let sameLevelBlockIdx = -1;
    for (let i = lastRootVisIdx + 1; i < visibleBlocks.length; i++) {
      if (visibleBlocks[i].level === minLevel) {
        sameLevelBlockIdx = i;
        break;
      }
    }
    if (sameLevelBlockIdx === -1) return;

    const sameLevelBlock = visibleBlocks[sameLevelBlockIdx];
    const sameLevelGroup = this.state.getBlockWithChildren(sameLevelBlock.id);
    const lastDescendant = sameLevelGroup[sameLevelGroup.length - 1];

    try {
      const moved = await this.api.moveBlocks(allSelectedIds, lastRoot.pageId, lastDescendant.id);
      this.state.replaceBlocks(allSelectedIds, moved, lastDescendant.id);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error('Failed to move selected blocks down:', e);
    }
  }

  private async indentSelectedBlocks(): Promise<void> {
    const selectedIds = this.state.selectedBlockIds();
    const allBlocks = this.state.blocks();
    const blocksToIndent = allBlocks.filter(b => selectedIds.has(b.id));

    if (blocksToIndent.length === 0) return;
    const firstIdx = allBlocks.findIndex(b => b.id === blocksToIndent[0].id);
    if (firstIdx <= 0) return;

    this.undo.begin('indent-selection');
    for (const b of blocksToIndent) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + 1 });
      this.state.updateBlock(b.id, { level: b.level + 1 });
    }
    this.undo.commit();

    try {
      await Promise.all(blocksToIndent.map(b => this.api.updateBlock(b.id, { level: b.level + 1 })));
    } catch (e) {
      console.error('Failed to indent selected blocks:', e);
    }
  }

  private async outdentSelectedBlocks(): Promise<void> {
    const selectedIds = this.state.selectedBlockIds();
    const allBlocks = this.state.blocks();
    const blocksToOutdent = allBlocks.filter(b => selectedIds.has(b.id));

    if (blocksToOutdent.some(b => b.level <= 1)) return;

    this.undo.begin('outdent-selection');
    for (const b of blocksToOutdent) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level - 1 });
      this.state.updateBlock(b.id, { level: b.level - 1 });
    }
    this.undo.commit();

    try {
      await Promise.all(blocksToOutdent.map(b => this.api.updateBlock(b.id, { level: b.level - 1 })));
    } catch (e) {
      console.error('Failed to outdent selected blocks:', e);
    }
  }

  private async copySelectedBlocks(): Promise<void> {
    const orderedIds = this.getOrderedSelectedBlockIds();
    if (orderedIds.length === 0) return;

    try {
      const result = await this.api.exportTreeMarkdown(orderedIds);

      // Write to system clipboard
      const clipboardItem = new ClipboardItem({
        'text/plain': new Blob([result.treeMarkdown], { type: 'text/plain' }),
        'text/html': new Blob([result.html], { type: 'text/html' }),
      });
      await navigator.clipboard.write([clipboardItem]);

      // Store flat markdown in memory
      this.clipboard.store(result.flatMarkdown, result.treeMarkdown);
    } catch (e) {
      console.error('Failed to copy selected blocks:', e);
    }
  }

  private async cutSelectedBlocks(): Promise<void> {
    await this.copySelectedBlocks();
    await this.deleteSelectedBlocks();
  }

  private onSelectionCopy(e: ClipboardEvent): void {
    if (!this.state.blockSelectionMode()) return;
    e.preventDefault();
    this.copySelectedBlocks();
  }

  private onSelectionCut(e: ClipboardEvent): void {
    if (!this.state.blockSelectionMode()) return;
    e.preventDefault();
    this.cutSelectedBlocks();
  }

  async performUndo(): Promise<void> {
    if (this.undo.isPending()) return;

    // Cancel pending auto-save for the editing block
    const editingId = this.state.focusedBlockId();
    if (editingId && this.dirtyBlockIds.has(editingId)) {
      this.dirtyBlockIds.delete(editingId);
      this.autoSave.cancelBlock(editingId);
    }

    const entry = this.undo.popUndo();
    if (!entry) return;

    // Apply inverse ops in reverse order
    for (let i = entry.ops.length - 1; i >= 0; i--) {
      const op = entry.ops[i];
      switch (op.type) {
        case 'update':
          this.state.updateBlock(op.before.id, op.before);
          if (op.before.collapsed !== op.after.collapsed) {
            this.state.toggleCollapsed(op.before.id);
          }
          break;
        case 'create':
          this.state.deleteBlock(op.block.id);
          break;
        case 'delete':
          this.state.addBlock(op.block, this.findInsertionPoint(op.block));
          break;
      }
    }

    // Restore cursor
    if (entry.cursorBefore) {
      this.state.addEditingBlock(entry.cursorBefore.blockId);
      this.state.setFocusedBlock(entry.cursorBefore.blockId, entry.cursorBefore.position);
      this.focusEditor(entry.cursorBefore.blockId);
    } else {
      // For body edits and other entries without explicit cursor, keep editing the first updated block
      const firstUpdate = entry.ops.find(op => op.type === 'update');
      if (firstUpdate && firstUpdate.type === 'update') {
        const pos = firstUpdate.before.body.length;
        this.state.addEditingBlock(firstUpdate.before.id);
        this.state.setFocusedBlock(firstUpdate.before.id, pos);
        this.focusEditor(firstUpdate.before.id);
      } else {
        this.state.removeAllEditingBlocks();
      }
    }

    this.undo.pushRedo(entry);
    this.syncToServer(entry, 'undo');
  }

  async performRedo(): Promise<void> {
    if (this.undo.isPending()) return;

    // Cancel pending auto-save for the editing block
    const editingId = this.state.focusedBlockId();
    if (editingId && this.dirtyBlockIds.has(editingId)) {
      this.dirtyBlockIds.delete(editingId);
      this.autoSave.cancelBlock(editingId);
    }

    const entry = this.undo.popRedo();
    if (!entry) return;

    // Apply forward ops in order
    for (const op of entry.ops) {
      switch (op.type) {
        case 'update':
          this.state.updateBlock(op.after.id, op.after);
          if (op.before.collapsed !== op.after.collapsed) {
            this.state.toggleCollapsed(op.after.id);
          }
          break;
        case 'create':
          this.state.addBlock(op.block, this.findInsertionPoint(op.block));
          break;
        case 'delete':
          this.state.deleteBlock(op.block.id);
          break;
      }
    }

    // Restore cursor
    if (entry.cursorAfter) {
      this.state.addEditingBlock(entry.cursorAfter.blockId);
      this.state.setFocusedBlock(entry.cursorAfter.blockId, entry.cursorAfter.position);
      this.focusEditor(entry.cursorAfter.blockId);
    } else {
      const firstUpdate = entry.ops.find(op => op.type === 'update');
      if (firstUpdate && firstUpdate.type === 'update') {
        const pos = firstUpdate.after.body.length;
        this.state.addEditingBlock(firstUpdate.after.id);
        this.state.setFocusedBlock(firstUpdate.after.id, pos);
        this.focusEditor(firstUpdate.after.id);
      } else {
        this.state.removeAllEditingBlocks();
      }
    }

    this.undo.pushUndo(entry);
    this.syncToServer(entry, 'redo');
  }

  private findInsertionPoint(block: Block): string {
    const blocks = this.state.blocks();
    let bestId = '';
    for (const b of blocks) {
      if (b.pageId === block.pageId && b.idx < block.idx) {
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
          await this.api.updateBlock(target.id, {
            body: target.body,
            level: target.level,
            collapsed: target.collapsed,
          });
          if (target.status !== source.status) {
            await this.api.updateBlockStatus(target.id, target.status);
          }
        } else if (op.type === 'create') {
          if (dir === 'undo') {
            // Undo a create = delete
            await this.api.deleteBlock(op.block.id);
          } else {
            // Redo a create = recreate
            const afterId = this.findInsertionPoint(op.block);
            const newBlock = await this.api.createBlock(
              op.block.pageId, afterId, op.block.level, op.block.body
            );
            // Remap: update local state with real server block
            this.state.deleteBlock(op.block.id);
            this.state.addBlock(newBlock, afterId);
            op.block = newBlock;
          }
        } else if (op.type === 'delete') {
          if (dir === 'undo') {
            // Undo a delete = recreate
            const afterId = this.findInsertionPoint(op.block);
            const newBlock = await this.api.createBlock(
              op.block.pageId, afterId, op.block.level, op.block.body
            );
            if (op.block.status !== 'normal') {
              await this.api.updateBlockStatus(newBlock.id, op.block.status);
            }
            // Remap
            this.state.deleteBlock(op.block.id);
            this.state.addBlock(newBlock, afterId);
            op.block = newBlock;
          } else {
            // Redo a delete = delete
            await this.api.deleteBlock(op.block.id);
          }
        }
      }
    } catch (e) {
      console.error('Failed to sync undo/redo to server:', e);
    }
  }

  private exitSelectionMode(): void {
    this.state.clearSelection();
    this.selectionAnchorBlockId = null;
    this.selectionExtentBlockId = null;
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

  private focusEditor(blockId?: string): void {
    afterNextRender(() => {
      const selector = blockId
        ? `#block-editor-${blockId}`
        : 'app-block-editor textarea';
      const el = document.querySelector<HTMLTextAreaElement>(selector);
      el?.focus();
    }, { injector: this.injector });
  }
}
