import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild } from '@angular/core';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { OutlineBlockComponent } from './outline-block';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { Block } from '../../core/models/block.model';
import { formatJournalDate } from '../../core/utils/date.utils';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, OutlineBlockComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport [itemSize]="40" class="outline-viewport" (scrolledIndexChange)="onScrolledIndexChange($event)">
      <div
        *cdkVirtualFor="let block of state.visibleBlocks(); let i = index; trackBy: trackById"
        class="outline-item"
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
export class OutlineComponent {
  readonly state = inject(OutlineStateService);
  private readonly api = inject(ApiService);
  private readonly autoSave = inject(AutoSaveService);
  private readonly injector = inject(Injector);
  readonly pageId = input.required<string>();
  readonly disableAutoLoad = input<boolean>(false);  // Disable automatic block loading
  readonly currentPageDate = input<string>('');  // Date shown in page header
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  readonly topBlockChanged = output<string>();  // Emits fileName of top visible block
  private readonly dirtyBlockIds = new Set<string>();
  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');

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
    // Check if fileName matches journal date format (YYYY-MM-DD)
    const datePattern = /^\d{4}-\d{2}-\d{2}$/;
    return datePattern.test(fileName);
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
    // Clear editing immediately so any blur during await won't match
    this.state.setEditingBlock(null);

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
          this.state.deleteBlocks(groupIds);
          this.state.addBlocks(moved, lastChildId);
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
      this.state.deleteBlocks(groupIds);
      this.state.addBlocks(moved, prevPrevId || '');
      this.focusEditor();
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

    if (sameLevelBlockIdx === -1) return; // No same-level block found above

    // Find the block with children to determine the actual position
    const allBlocks = this.state.blocks();
    const sameLevelBlock = blocks[sameLevelBlockIdx];
    const sameLevelIdx = allBlocks.findIndex(b => b.id === sameLevelBlock.id);

    // Move to just before the same-level block (considering its children)
    const targetPrev = sameLevelIdx > 0 ? allBlocks[sameLevelIdx - 1] : undefined;
    const targetPrevId = targetPrev?.id ?? '';

    const moved = await this.api.moveBlocks(groupIds, block.pageId, targetPrevId);
    this.state.deleteBlocks(groupIds);
    this.state.addBlocks(moved, targetPrevId);
    this.focusEditor();
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

    // Standard placement: move entire tree after the entire next tree
    if (this.canPlace(block.level, targetBlock)) {
      // Get the target block and all its children to find the bottom of the next tree
      const targetGroup = this.state.getBlockWithChildren(targetBlock.id);
      const lastTargetDescendant = targetGroup[targetGroup.length - 1];

      // Normal move down - move after the entire next tree
      const moved = await this.api.moveBlocks(groupIds, block.pageId, lastTargetDescendant.id);
      this.state.deleteBlocks(groupIds);
      this.state.addBlocks(moved, lastTargetDescendant.id);
      this.focusEditor();
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

    if (sameLevelBlockIdx === -1) return; // No same-level block found below

    // Get the same-level block and all its children
    const sameLevelBlock = blocks[sameLevelBlockIdx];
    const sameLevelGroup = this.state.getBlockWithChildren(sameLevelBlock.id);
    const lastDescendant = sameLevelGroup[sameLevelGroup.length - 1];

    // Move to just after the same-level block and all its children
    const moved = await this.api.moveBlocks(groupIds, block.pageId, lastDescendant.id);
    this.state.deleteBlocks(groupIds);
    this.state.addBlocks(moved, lastDescendant.id);
    this.focusEditor();
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
    const parsedBlocks = this.parseTextIntoBlocks(event.text);

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
