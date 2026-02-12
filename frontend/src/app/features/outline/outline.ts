import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild, OnDestroy } from '@angular/core';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { OutlineBlockComponent } from './outline-block';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { Block } from '../../core/models/block.model';
import { ClipboardService } from '../../core/services/clipboard.service';
import { formatJournalDate } from '../../core/utils/date.utils';
import { isJournalFileName } from '../../core/utils/journal-filename.utils';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, OutlineBlockComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport [itemSize]="40" class="outline-viewport" (scrolledIndexChange)="onScrolledIndexChange($event)">
      <div
        *cdkVirtualFor="let block of state.visibleBlocks(); let i = index; trackBy: trackById"
        class="outline-item"
        (mouseenter)="onBlockMouseEnter(block)"
      >
        @if (isFirstBlockOfDate(i) && block.fileName !== currentPageDate() && isJournalDate(block.fileName)) {
          <div class="date-header">
            {{ formatDateHeader(block.fileName) }}
          </div>
        }
        <app-outline-block
          [block]="block"
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
          (taskToggled)="onCheckboxToggled(block)"
          (statusCycleRequested)="onStatusCycleRequested(block)"
          (collapseToggled)="onCollapseToggled(block)"
          (pasteDetected)="onPaste(block, $event)"
          (dragBeyondBounds)="onDragBeyondBounds(block, $event)"
        />
      </div>
    </cdk-virtual-scroll-viewport>
  `,
  styles: [`
    :host { display: block; height: 100%; width: 100%; }
    .outline-viewport {
      height: 100%;
      width: 100% !important;
    }
    .outline-viewport ::ng-deep .cdk-virtual-scroll-content-wrapper {
      width: 100%;
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
  `]
})
export class OutlineComponent implements OnDestroy {
  readonly state = inject(OutlineStateService);
  private readonly api = inject(ApiService);
  private readonly autoSave = inject(AutoSaveService);
  private readonly injector = inject(Injector);
  private readonly clipboard = inject(ClipboardService);
  readonly pageId = input.required<string>();
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic block loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  readonly topBlockChanged = output<string>();  // Emits fileName of top visible block
  private readonly dirtyBlockIds = new Set<string>();
  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');
  private selectionAnchorBlockId: string | null = null;
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

  }

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

  trackById(_index: number, block: Block): string {
    return block.id;
  }

  isFirstBlockOfDate(index: number): boolean {
    const blocks = this.state.visibleBlocks();
    if (index === 0) return true;
    const currentBlock = blocks[index];
    const previousBlock = blocks[index - 1];
    return currentBlock.fileName !== previousBlock.fileName;
  }

  isJournalDate(fileName: string): boolean {
    return isJournalFileName(fileName);
  }

  formatDateHeader(fileName: string): string {
    return formatJournalDate(fileName);
  }

  private async loadBlocks(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPage(pageId);
      this.state.setBlocks(result.blocks);
    } catch (e) {
      console.error('Failed to load blocks for page:', pageId, e);
    }
  }

  onEditRequested(blockId: string, cursorPosition: number): void {
    if (this.state.blockSelectionMode()) return;
    this.state.setEditingBlock(blockId, cursorPosition);
  }

  onBodyChanged(block: Block, newBody: string): void {
    this.state.updateBlock(block.id, { body: newBody });
    this.dirtyBlockIds.add(block.id);
    this.autoSave.scheduleBlockUpdate(block.id, newBody);
  }

  async onEditFinished(block: Block): Promise<void> {
    // If editing has already moved to a different block (e.g. Enter/Backspace/Delete),
    // don't clear it — the blur is from the old block-editor being removed from DOM.
    if (this.state.editingBlockId() !== block.id) return;

    if (!this.dirtyBlockIds.has(block.id)) {
      this.state.setEditingBlock(null);
      return;
    }
    this.dirtyBlockIds.delete(block.id);
    await this.autoSave.flushBlock(block.id);
    const current = this.state.getBlockById(block.id);
    if (current) {
      const updated = await this.api.updateBlock(block.id, { body: current.body });
      this.state.updateBlock(block.id, { html: updated.html });
      // Switch to view mode AFTER HTML is updated (prevents flash)
      this.state.setEditingBlock(null);
    } else {
      this.state.setEditingBlock(null);
    }
  }

  async onEnterPressed(block: Block, event: { cursorPos: number }): Promise<void> {
    this.autoSave.cancelBlock(block.id);
    this.dirtyBlockIds.delete(block.id);
    // Suppress duplicate animation — block content changes immediately

    // Clear editing immediately so any blur during await won't match
    this.state.setEditingBlock(null);

    try {
      const { cursorPos } = event;
      const beforeText = block.body.substring(0, cursorPos);
      const afterText = block.body.substring(cursorPos);

      if (block.body.length === 0 && block.level > 0) {
        // Empty block with indent: outdent instead of creating new block
        this.state.setEditingBlock(block.id);
        await this.onOutdent(block);
        this.focusEditor(block.id);
        return;
      }

      if (cursorPos === 0 && block.body.length > 0) {
        // Enter at beginning: create new empty block before current block
        const allBlocks = this.state.blocks();
        const idx = allBlocks.findIndex(b => b.id === block.id);

        // Find the block just before the current block
        const prevBlockId = idx > 0 ? allBlocks[idx - 1].id : '';

        // Create new empty block with same level as current block
        const newBlock = await this.api.createBlock(block.pageId, prevBlockId, block.level, '');
        this.state.addBlock(newBlock, prevBlockId);

        // Focus the new block
        this.state.setEditingBlock(newBlock.id);
        this.focusEditor(newBlock.id);
      } else {
        this.state.updateBlock(block.id, { body: beforeText });
        const updatedOriginal = await this.api.updateBlock(block.id, { body: beforeText });
        this.state.updateBlock(block.id, { html: updatedOriginal.html });
        const newBlock = await this.api.createBlock(block.pageId, block.id, block.level, afterText);
        this.state.addBlock(newBlock, block.id);
        this.state.setEditingBlock(newBlock.id);
        this.focusEditor(newBlock.id);
      }
    } finally {

    }
  }

  async onBackspaceAtStart(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;

    this.autoSave.cancelBlock(block.id);
    this.dirtyBlockIds.delete(block.id);


    const prevBlock = blocks[idx - 1];
    const mergedBody = prevBlock.body + block.body;
    this.state.updateBlock(prevBlock.id, { body: mergedBody });
    this.state.deleteBlock(block.id);
    this.state.setEditingBlock(prevBlock.id);
    this.focusEditor(prevBlock.id);
    // Reset after microtask — DOM removal (and blur) may be deferred by virtual scroll

    const updatedPrev = await this.api.updateBlock(prevBlock.id, { body: mergedBody });
    this.state.updateBlock(prevBlock.id, { html: updatedPrev.html });
    await this.api.deleteBlock(block.id);
  }

  async onDeleteAtEnd(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx >= blocks.length - 1) return;

    const nextBlock = blocks[idx + 1];
    this.autoSave.cancelBlock(nextBlock.id);
    this.dirtyBlockIds.delete(nextBlock.id);


    const mergedBody = block.body + nextBlock.body;
    this.state.updateBlock(block.id, { body: mergedBody });
    this.state.deleteBlock(nextBlock.id);

    const updatedBlock = await this.api.updateBlock(block.id, { body: mergedBody });
    this.state.updateBlock(block.id, { html: updatedBlock.html });
    await this.api.deleteBlock(nextBlock.id);
  }

  async onNavigateUp(block: Block, cursorPos: number): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx > 0) {
      // Save and update HTML before navigating
      await this.saveBlockIfDirty(block);
      this.state.setEditingBlock(blocks[idx - 1].id, cursorPos);
    }
  }

  async onNavigateDown(block: Block, cursorPos: number): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx < blocks.length - 1) {
      // Save and update HTML before navigating
      await this.saveBlockIfDirty(block);
      this.state.setEditingBlock(blocks[idx + 1].id, cursorPos);
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
    if (!prevBlock) return blockLevel === 0;
    return prevBlock.level === blockLevel || prevBlock.level === blockLevel - 1;
  }

  async onIndent(block: Block): Promise<void> {
    const blocks = this.state.blocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;
    const prevBlock = blocks[idx - 1];
    const newLevel = block.level + 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevBlock)) {
      // Exception: indenting by 1 level is allowed if there is a block at the same level before
      let hasSameLevelBefore = false;
      for (let i = idx - 1; i >= 0; i--) {
        if (blocks[i].level === block.level) {
          hasSameLevelBefore = true;
          break;
        }
      }
      if (!hasSameLevelBefore) return;
    }

    const group = this.state.getBlockWithChildren(block.id);
    for (const b of group) {
      this.state.updateBlock(b.id, { level: b.level + 1 });
    }
    await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level + 1 })));
  }

  async onOutdent(block: Block): Promise<void> {
    if (block.level <= 0) return;
    const blocks = this.state.blocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    const prevBlock = idx > 0 ? blocks[idx - 1] : undefined;
    const newLevel = block.level - 1;

    // Check normal placement
    if (!this.canPlace(newLevel, prevBlock)) {
      // Exception: outdenting by 1 level is allowed by placing after the last child of the current parent
      // Find the current parent (the block with level = current level - 1 before the current block)
      let currentParentIdx = -1;
      for (let i = idx - 1; i >= 0; i--) {
        if (blocks[i].level === block.level - 1) {
          currentParentIdx = i;
          break;
        }
      }

      if (currentParentIdx >= 0) {
        const currentParent = blocks[currentParentIdx];

        // Get ALL children of the current parent (including the block being moved)
        const allChildren = this.state.getChildren(currentParent.id);

        // Find the last child (excluding the block being moved and its descendants)
        const group = this.state.getBlockWithChildren(block.id);
        const movingIds = new Set(group.map(b => b.id));

        let lastChildId = currentParent.id;
        for (const child of allChildren) {
          if (!movingIds.has(child.id)) {
            lastChildId = child.id;
          }
        }

        // Allow outdent by placing after the last child
        const groupIds = group.map(b => b.id);

        // Update levels
        for (const b of group) {
          this.state.updateBlock(b.id, { level: b.level - 1 });
        }
        await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level - 1 })));

        // Move after the last child (if it's not the parent itself)
        if (lastChildId !== currentParent.id) {
          const moved = await this.api.moveBlocks(groupIds, block.pageId, lastChildId);
          this.state.replaceBlocks(groupIds, moved, lastChildId);
        }
        return;
      }

      return; // Cannot place
    }

    // Normal outdent
    const group = this.state.getBlockWithChildren(block.id);
    for (const b of group) {
      this.state.updateBlock(b.id, { level: b.level - 1 });
    }
    await Promise.all(group.map(b => this.api.updateBlock(b.id, { level: b.level - 1 })));
  }

  async onMoveUp(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;

    const group = this.state.getBlockWithChildren(block.id);
    const groupIds = group.map(b => b.id);

    // Suppress duplicate animation during move — block reappears at new position

    this.state.setEditingBlock(null);

    try {
      // Standard placement: move entire tree before the entire previous tree
      const prevBlock = blocks[idx - 1];
      if (this.canPlace(block.level, prevBlock)) {
        // Find the previous block's position in allBlocks to move before its entire tree
        const allBlocks = this.state.blocks();
        const prevBlockIdx = allBlocks.findIndex(b => b.id === prevBlock.id);
        const targetPrev = prevBlockIdx > 0 ? allBlocks[prevBlockIdx - 1] : undefined;
        const prevPrevId = targetPrev?.id ?? '';

        // Normal move up - move before the entire previous tree
        const moved = await this.api.moveBlocks(groupIds, block.pageId, prevPrevId);
        this.state.replaceBlocks(groupIds, moved, prevPrevId || '');
        this.state.setEditingBlock(block.id);
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
        // No same-level block found above — restore editing
        this.state.setEditingBlock(block.id);
        return;
      }

      // Find the block with children to determine the actual position
      const allBlocks = this.state.blocks();
      const sameLevelBlock = blocks[sameLevelBlockIdx];
      const sameLevelIdx = allBlocks.findIndex(b => b.id === sameLevelBlock.id);

      // Move to just before the same-level block (considering its children)
      const targetPrev = sameLevelIdx > 0 ? allBlocks[sameLevelIdx - 1] : undefined;
      const targetPrevId = targetPrev?.id ?? '';

      const moved = await this.api.moveBlocks(groupIds, block.pageId, targetPrevId);
      this.state.replaceBlocks(groupIds, moved, targetPrevId);
      this.state.setEditingBlock(block.id);
      this.focusEditor(block.id);
    } finally {

    }
  }

  async onMoveDown(block: Block): Promise<void> {
    const blocks = this.state.visibleBlocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx >= blocks.length - 1) return;

    const children = this.state.getChildren(block.id);
    const group = [block, ...children];
    const groupIds = group.map(b => b.id);

    // Find the block after the entire group in visible blocks
    const allBlocks = this.state.blocks();
    const lastChildIdx = allBlocks.findIndex(b => b.id === group[group.length - 1].id);
    if (lastChildIdx >= allBlocks.length - 1) return;
    const targetBlock = allBlocks[lastChildIdx + 1];

    // Suppress duplicate animation during move — block reappears at new position

    this.state.setEditingBlock(null);

    try {
      // Standard placement: move entire tree after the entire next tree
      if (this.canPlace(block.level, targetBlock)) {
        // Get the target block and all its children to find the bottom of the next tree
        const targetGroup = this.state.getBlockWithChildren(targetBlock.id);
        const lastTargetDescendant = targetGroup[targetGroup.length - 1];

        // Normal move down - move after the entire next tree
        const moved = await this.api.moveBlocks(groupIds, block.pageId, lastTargetDescendant.id);
        this.state.replaceBlocks(groupIds, moved, lastTargetDescendant.id);
        this.state.setEditingBlock(block.id);
        this.focusEditor(block.id);
        return;
      }

      // Exception: If there's a block at the same level below, move just after it (and its children)
      let sameLevelBlockIdx = -1;
      for (let i = idx + 1; i < blocks.length; i++) {
        if (blocks[i].level === block.level) {
          sameLevelBlockIdx = i;
          break;
        }
      }

      if (sameLevelBlockIdx === -1) {
        // No same-level block found below — restore editing
        this.state.setEditingBlock(block.id);
        return;
      }

      // Get the same-level block and all its children
      const sameLevelBlock = blocks[sameLevelBlockIdx];
      const sameLevelGroup = this.state.getBlockWithChildren(sameLevelBlock.id);
      const lastDescendant = sameLevelGroup[sameLevelGroup.length - 1];

      // Move to just after the same-level block and all its children
      const moved = await this.api.moveBlocks(groupIds, block.pageId, lastDescendant.id);
      this.state.replaceBlocks(groupIds, moved, lastDescendant.id);
      this.state.setEditingBlock(block.id);
      this.focusEditor(block.id);
    } finally {

    }
  }

  async onCheckboxToggled(block: Block): Promise<void> {
    try {
      // Checkbox click: todo → running → done (stop there)
      let newStatus = block.status;
      if (block.status === 'todo') {
        newStatus = 'running';
      } else if (block.status === 'running') {
        newStatus = 'done';
      }
      if (newStatus !== block.status) {
        const updated = await this.api.updateBlockStatus(block.id, newStatus);
        this.state.updateBlock(block.id, { status: updated.status });
      }
    } catch (e) {
      console.error('Failed to toggle task status:', e);
    }
  }

  async onStatusCycleRequested(block: Block): Promise<void> {
    try {
      // Ctrl+Enter: normal → todo → running → done → normal
      const cycle = ['normal', 'todo', 'running', 'done'];
      const currentIndex = cycle.indexOf(block.status);
      const newStatus = cycle[(currentIndex + 1) % cycle.length];
      const updated = await this.api.updateBlockStatus(block.id, newStatus);
      this.state.updateBlock(block.id, { status: updated.status });
    } catch (e) {
      console.error('Failed to cycle status:', e);
    }
  }

  async onCollapseToggled(block: Block): Promise<void> {
    try {
      const newCollapsed = !block.collapsed;
      // Update local state immediately for responsive UI
      this.state.toggleCollapsed(block.id);
      this.state.updateBlock(block.id, { collapsed: newCollapsed });
      // Persist to backend
      await this.api.updateBlock(block.id, { collapsed: newCollapsed });
    } catch (e) {
      console.error('Failed to toggle collapse:', e);
      // Revert on error
      this.state.toggleCollapsed(block.id);
      this.state.updateBlock(block.id, { collapsed: block.collapsed });
    }
  }

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

    // Split existing text at caret position
    const before = block.body.substring(0, event.cursorPos);
    const after = block.body.substring(event.cursorPos);

    // Merge existing text around pasted content
    parsedBlocks[0].body = before + parsedBlocks[0].body;
    parsedBlocks[parsedBlocks.length - 1].body = parsedBlocks[parsedBlocks.length - 1].body + after;

    try {
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

      // Add remaining blocks after the current block with level 0
      if (parsedBlocks.length > 1) {
        let prevBlockId = block.id;
        for (let i = 1; i < parsedBlocks.length; i++) {
          const parsedBlock = parsedBlocks[i];
          const newBlock = await this.api.createBlock(
            block.pageId,
            prevBlockId,
            0, // level is always 0
            parsedBlock.body
          );
          this.state.addBlocks([newBlock], prevBlockId);

          // Update status if needed
          if (parsedBlock.status !== 'normal') {
            await this.api.updateBlockStatus(newBlock.id, parsedBlock.status);
            this.state.updateBlock(newBlock.id, { status: parsedBlock.status });
          }

          prevBlockId = newBlock.id;
        }
      }

      // Focus the last created block or the current block
      this.focusEditor();
    } catch (e) {
      console.error('Failed to process paste:', e);
    }
  }

  private parseTextIntoBlocks(text: string): Array<{ body: string; status: string }> {
    const blocks: Array<{ body: string; status: string }> = [];
    const lines = text.split('\n');
    let currentParagraph: string[] = [];
    let blankLineCount = 0;

    for (const line of lines) {
      const trimmedLine = line.trim();

      // Check if line is blank
      if (trimmedLine === '') {
        blankLineCount++;
        if (currentParagraph.length > 0 && blankLineCount === 1) {
          // Keep the blank line as part of the paragraph
          currentParagraph.push(line);
        }
        continue;
      }

      // Check if line starts with list markers (after removing leading whitespace)
      const lineWithoutLeadingSpace = line.trimStart();
      let isListItem = false;
      let listItemBody = '';
      let listItemStatus = 'normal';

      if (lineWithoutLeadingSpace.startsWith('- [x] ')) {
        isListItem = true;
        listItemBody = lineWithoutLeadingSpace.substring(6); // Remove "- [x] "
        listItemStatus = 'done';
      } else if (lineWithoutLeadingSpace.startsWith('- [ ] ')) {
        isListItem = true;
        listItemBody = lineWithoutLeadingSpace.substring(6); // Remove "- [ ] "
        listItemStatus = 'todo';
      } else if (lineWithoutLeadingSpace.startsWith('- ')) {
        isListItem = true;
        listItemBody = lineWithoutLeadingSpace.substring(2); // Remove "- "
        listItemStatus = 'normal';
      }

      // If we have one or more blank lines, flush the current paragraph
      if (blankLineCount >= 1 && currentParagraph.length > 0) {
        // Remove trailing blank lines from the paragraph
        while (currentParagraph.length > 0 && currentParagraph[currentParagraph.length - 1].trim() === '') {
          currentParagraph.pop();
        }
        if (currentParagraph.length > 0) {
          blocks.push({ body: currentParagraph.join('\n'), status: 'normal' });
          currentParagraph = [];
        }
      }

      // If this is a list item, flush current paragraph and add as new block
      if (isListItem) {
        if (currentParagraph.length > 0) {
          // Remove trailing blank lines
          while (currentParagraph.length > 0 && currentParagraph[currentParagraph.length - 1].trim() === '') {
            currentParagraph.pop();
          }
          if (currentParagraph.length > 0) {
            blocks.push({ body: currentParagraph.join('\n'), status: 'normal' });
            currentParagraph = [];
          }
        }
        blocks.push({ body: listItemBody, status: listItemStatus });
        blankLineCount = 0;
      } else {
        // Regular line, add to current paragraph
        currentParagraph.push(line);
        blankLineCount = 0;
      }
    }

    // Flush any remaining paragraph
    if (currentParagraph.length > 0) {
      // Remove trailing blank lines
      while (currentParagraph.length > 0 && currentParagraph[currentParagraph.length - 1].trim() === '') {
        currentParagraph.pop();
      }
      if (currentParagraph.length > 0) {
        blocks.push({ body: currentParagraph.join('\n'), status: 'normal' });
      }
    }

    return blocks;
  }

  onDragBeyondBounds(block: Block, direction: 'above' | 'below'): void {
    // Exit edit mode
    this.state.setEditingBlock(null);
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

  onBlockMouseEnter(block: Block): void {
    if (!this.isSelectionDragging) return;

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
    // Check if click is on a selected block
    const target = e.target as HTMLElement;
    const blockEl = target.closest('.outline-block.selected');
    if (!blockEl) {
      this.exitSelectionMode();
    }
  }

  handleSelectionKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.exitSelectionMode();
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

    this.exitSelectionMode();

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

    // Collect all selected block IDs (roots + children) in order
    const allSelectedIds: string[] = [];
    for (const root of rootBlocks) {
      const group = this.state.getBlockWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }

    const allBlocks = this.state.blocks();

    if (this.canPlace(minLevel, prevBlock)) {
      // Find target: place before the previous block
      const prevBlockAllIdx = allBlocks.findIndex(b => b.id === prevBlock.id);
      const targetPrev = prevBlockAllIdx > 0 ? allBlocks[prevBlockAllIdx - 1] : undefined;
      const targetPrevId = targetPrev?.id ?? '';

      try {
        const moved = await this.api.moveBlocks(allSelectedIds, firstRoot.pageId, targetPrevId);
        this.state.replaceBlocks(allSelectedIds, moved, targetPrevId);
      } catch (e) {
        console.error('Failed to move selected blocks up:', e);
      }
      return;
    }

    // Exception: find same-level block above
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

    // Collect all selected block IDs in order
    const allSelectedIds: string[] = [];
    for (const root of rootBlocks) {
      const group = this.state.getBlockWithChildren(root.id);
      group.forEach(b => allSelectedIds.push(b.id));
    }

    if (this.canPlace(minLevel, nextBlock)) {
      const nextGroup = this.state.getBlockWithChildren(nextBlock.id);
      const targetAfter = nextGroup[nextGroup.length - 1];

      try {
        const moved = await this.api.moveBlocks(allSelectedIds, lastRoot.pageId, targetAfter.id);
        this.state.replaceBlocks(allSelectedIds, moved, targetAfter.id);
      } catch (e) {
        console.error('Failed to move selected blocks down:', e);
      }
      return;
    }

    // Exception: find same-level block below
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
    } catch (e) {
      console.error('Failed to move selected blocks down:', e);
    }
  }

  private async indentSelectedBlocks(): Promise<void> {
    const selectedIds = this.state.selectedBlockIds();
    const allBlocks = this.state.blocks();
    const blocksToIndent = allBlocks.filter(b => selectedIds.has(b.id));

    // Check: first selected block must have a block before it
    if (blocksToIndent.length === 0) return;
    const firstIdx = allBlocks.findIndex(b => b.id === blocksToIndent[0].id);
    if (firstIdx <= 0) return;

    // Update state
    for (const b of blocksToIndent) {
      this.state.updateBlock(b.id, { level: b.level + 1 });
    }

    // Update backend
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

    // Check: all blocks must have level > 0
    if (blocksToOutdent.some(b => b.level <= 0)) return;

    // Update state
    for (const b of blocksToOutdent) {
      this.state.updateBlock(b.id, { level: b.level - 1 });
    }

    // Update backend
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

  exitSelectionMode(): void {
    this.state.clearSelection();
    this.selectionAnchorBlockId = null;
    this.isSelectionDragging = false;
    document.removeEventListener('mouseup', this.selectionMouseUpHandler);
    document.removeEventListener('keydown', this.selectionKeydownHandler);
    document.removeEventListener('click', this.selectionClickHandler);
    document.removeEventListener('copy', this.selectionCopyHandler);
    document.removeEventListener('cut', this.selectionCutHandler);
  }

  ngOnDestroy(): void {
    document.removeEventListener('mouseup', this.selectionMouseUpHandler);
    document.removeEventListener('keydown', this.selectionKeydownHandler);
    document.removeEventListener('click', this.selectionClickHandler);
    document.removeEventListener('copy', this.selectionCopyHandler);
    document.removeEventListener('cut', this.selectionCutHandler);
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
