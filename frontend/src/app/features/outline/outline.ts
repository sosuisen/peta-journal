import { Component, inject, input, output, effect, Injector, afterNextRender, viewChild } from '@angular/core';
import { ScrollingModule, CdkVirtualScrollViewport } from '@angular/cdk/scrolling';
import { OutlineBlockComponent } from './outline-block';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { Block } from '../../core/models/block.model';

@Component({
  selector: 'app-outline',
  imports: [ScrollingModule, OutlineBlockComponent],
  template: `
    <cdk-virtual-scroll-viewport #viewport itemSize="28" class="outline-viewport" (scrolledIndexChange)="onScrolledIndexChange($event)">
      <div
        *cdkVirtualFor="let block of state.visibleBlocks(); trackBy: trackById"
        class="outline-item"
      >
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
  `]
})
export class OutlineComponent {
  readonly state = inject(OutlineStateService);
  private readonly api = inject(ApiService);
  private readonly autoSave = inject(AutoSaveService);
  private readonly injector = inject(Injector);
  readonly pageId = input.required<string>();
  readonly loadMore = output<{ filepath: string; currentIndex: number }>();  // Emits when should load more
  private readonly dirtyBlockIds = new Set<string>();
  private readonly viewport = viewChild<CdkVirtualScrollViewport>('viewport');

  constructor() {
    effect(() => {
      const id = this.pageId();
      if (id) this.loadBlocks(id);
    });

  }

  onScrolledIndexChange(index: number): void {
    const blocks = this.state.visibleBlocks();
    const totalBlocks = blocks.length;

    console.log('Scrolled index changed:', index, 'Total blocks:', totalBlocks);

    // Check if we need more buffer (100 blocks ahead)
    if (totalBlocks < index + 100 && totalBlocks > 0) {
      const lastBlock = blocks[totalBlocks - 1];
      const filepath = lastBlock.dirPath + lastBlock.fileName;
      console.log('Need more buffer! Current index:', index, 'Total:', totalBlocks);
      this.loadMore.emit({ filepath, currentIndex: index });
    }
  }

  trackById(_index: number, block: Block): string {
    return block.id;
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

    if (cursorPos === 0 && block.body.length > 0) {
      const newBlock = await this.api.createBlock(block.pageId, '', block.level, '');
      this.state.addBlock(newBlock, '');
      const idx = this.state.getBlockIndex(block.id);
      if (idx > 0) {
        const prevBlock = this.state.blocks()[idx - 1];
        this.state.addBlock(newBlock, prevBlock.id);
      }
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

  /**
   * Get the last descendant of a block (the last child in the entire subtree).
   */
  private getLastDescendant(blockId: string): Block | null {
    const children = this.state.getChildren(blockId);
    if (children.length === 0) return null;
    return children[children.length - 1];
  }

  async onIndent(block: Block): Promise<void> {
    const blocks = this.state.blocks();
    const idx = blocks.findIndex(b => b.id === block.id);
    if (idx <= 0) return;
    const prevBlock = blocks[idx - 1];
    const newLevel = block.level + 1;
    if (!this.canPlace(newLevel, prevBlock)) return;

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
          await this.api.moveBlocks(groupIds, block.pageId, lastChildId);
          this.state.deleteBlocks(groupIds);
          this.state.addBlocks(group.map(b => ({ ...b, level: b.level - 1 })), lastChildId);
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
    const newPrev = idx >= 2 ? blocks[idx - 2] : undefined;
    if (!this.canPlace(block.level, newPrev)) return;

    const group = this.state.getBlockWithChildren(block.id);
    const groupIds = group.map(b => b.id);
    const prevPrevId = newPrev?.id ?? '';
    await this.api.moveBlocks(groupIds, block.pageId, prevPrevId);
    this.state.deleteBlocks(groupIds);
    this.state.addBlocks(group, prevPrevId || '');
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
    if (!this.canPlace(block.level, targetBlock)) return;

    await this.api.moveBlocks(groupIds, block.pageId, targetBlock.id);
    this.state.deleteBlocks(groupIds);
    this.state.addBlocks(group, targetBlock.id);
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
