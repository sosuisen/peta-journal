import { Injectable, signal, computed } from '@angular/core';
import { Block } from '../models/block.model';

@Injectable({ providedIn: 'root' })
export class OutlineStateService {
  private readonly _blocks = signal<Block[]>([]);
  private readonly _focusedBlockId = signal<string | null>(null);
  private readonly _editingBlockIds = signal<Set<string>>(new Set());
  private readonly _selectedBlockIds = signal<Set<string>>(new Set());
  private readonly _collapsedBlockIds = signal<Set<string>>(new Set());
  private readonly _blockSelectionMode = signal<boolean>(false);
  private readonly _pendingCursorPosition = signal<number | null>(null);
  private readonly _pendingMouseDrag = signal<boolean>(false);
  readonly blocks = this._blocks.asReadonly();
  readonly focusedBlockId = this._focusedBlockId.asReadonly();
  readonly editingBlockIds = this._editingBlockIds.asReadonly();
  readonly pendingCursorPosition = this._pendingCursorPosition.asReadonly();
  readonly selectedBlockIds = this._selectedBlockIds.asReadonly();
  readonly blockSelectionMode = this._blockSelectionMode.asReadonly();

  readonly visibleBlocks = computed(() => {
    const blocks = this._blocks();
    const collapsed = this._collapsedBlockIds();
    const result: Block[] = [];
    let skipLevel = -1;

    for (const block of blocks) {
      if (skipLevel >= 0 && block.level > skipLevel) {
        continue;
      }
      skipLevel = -1;
      result.push(block);
      if (collapsed.has(block.id)) {
        skipLevel = block.level;
      }
    }
    return result;
  });

  /**
   * Replaces all blocks and rebuilds the collapsed block set from their collapsed flags.
   * @param blocks - The new complete list of blocks.
   */
  setBlocks(blocks: Block[]): void {
    this._blocks.set(blocks);
    const collapsed = new Set<string>();
    for (const b of blocks) {
      if (b.collapsed) collapsed.add(b.id);
    }
    this._collapsedBlockIds.set(collapsed);
  }

  /**
   * Inserts a single block after the specified block, or at the beginning if not found.
   * @param block - The block to insert.
   * @param afterBlockId - The ID of the block to insert after.
   */
  addBlock(block: Block, afterBlockId: string): void {
    this._blocks.update(blocks => {
      const idx = afterBlockId ? blocks.findIndex(b => b.id === afterBlockId) : -1;
      const newBlocks = [...blocks];
      newBlocks.splice(idx + 1, 0, block);
      return newBlocks;
    });
  }

  /**
   * Partially updates a block's properties by merging the given updates.
   * @param blockId - The ID of the block to update.
   * @param updates - The partial block properties to merge.
   */
  updateBlock(blockId: string, updates: Partial<Block>): void {
    this._blocks.update(blocks =>
      blocks.map(b => b.id === blockId ? { ...b, ...updates } : b)
    );
  }

  /**
   * Removes a single block by its ID.
   * @param blockId - The ID of the block to remove.
   */
  deleteBlock(blockId: string): void {
    this._blocks.update(blocks => blocks.filter(b => b.id !== blockId));
  }

  /**
   * Adds a block to the editing set (textarea visible).
   */
  addEditingBlock(blockId: string): void {
    this._editingBlockIds.update(s => { const n = new Set(s); n.add(blockId); return n; });
  }

  /**
   * Focuses a block and sets a pending cursor position for the next time the block is edited.
   */
  setFocusedBlock(blockId: string, cursorPosition: number | null = null): void {
    this._pendingCursorPosition.set(cursorPosition);
    this._focusedBlockId.set(blockId);
  }

  /**
   * Removes a block from the editing set (switches it to view mode).
   */
  removeEditingBlock(blockId: string): void {
    this._editingBlockIds.update(s => { const n = new Set(s); n.delete(blockId); return n; });
  }

  removeAllEditingBlocks(): void {
    this._editingBlockIds.set(new Set());
  }

  /**
   * Returns and clears the pending cursor position (one-time read).
   * @returns The pending cursor position, or null if none was set.
   */
  consumePendingCursorPosition(): number | null {
    const pos = this._pendingCursorPosition();
    this._pendingCursorPosition.set(null);
    return pos;
  }

  setPendingMouseDrag(value: boolean): void {
    this._pendingMouseDrag.set(value);
  }

  consumePendingMouseDrag(): boolean {
    const value = this._pendingMouseDrag();
    this._pendingMouseDrag.set(false);
    return value;
  }

  /**
   * Toggles the collapsed state of a block, hiding or showing its children.
   * @param blockId - The ID of the block to toggle.
   */
  toggleCollapsed(blockId: string): void {
    this._collapsedBlockIds.update(set => {
      const next = new Set(set);
      if (next.has(blockId)) next.delete(blockId);
      else next.add(blockId);
      return next;
    });
  }

  /**
   * Finds a block by its ID.
   * @param blockId - The ID of the block to find.
   * @returns The matching block, or undefined if not found.
   */
  getBlockById(blockId: string): Block | undefined {
    return this._blocks().find(b => b.id === blockId);
  }

  /**
   * Checks whether the block has any immediate children (next block at a deeper level).
   * @param blockId - The ID of the block to check.
   * @returns True if the block has children, false otherwise.
   */
  hasChildren(blockId: string): boolean {
    const blocks = this._blocks();
    const idx = blocks.findIndex(b => b.id === blockId);
    if (idx < 0 || idx === blocks.length - 1) return false;
    return blocks[idx + 1].level > blocks[idx].level;
  }

  /**
   * Get all descendant blocks of the given block.
   * Children are all subsequent blocks with a level strictly greater than the parent's level,
   * stopping at the first block at the same or lower level.
   */
  getChildren(blockId: string): Block[] {
    const blocks = this._blocks();
    const idx = blocks.findIndex(b => b.id === blockId);
    if (idx < 0) return [];
    const parentLevel = blocks[idx].level;
    const children: Block[] = [];
    for (let i = idx + 1; i < blocks.length; i++) {
      if (blocks[i].level <= parentLevel) break;
      children.push(blocks[i]);
    }
    return children;
  }

  /**
   * Get the block and all its descendants as a group.
   */
  getBlockWithChildren(blockId: string): Block[] {
    const block = this.getBlockById(blockId);
    if (!block) return [];
    return [block, ...this.getChildren(blockId)];
  }

  /**
   * Inserts multiple blocks after the specified block, or at the beginning if not found.
   * @param blocks - The blocks to insert.
   * @param afterBlockId - The ID of the block to insert after.
   */
  addBlocks(blocks: Block[], afterBlockId: string): void {
    this._blocks.update(current => {
      const idx = afterBlockId ? current.findIndex(b => b.id === afterBlockId) : -1;
      const newBlocks = [...current];
      newBlocks.splice(idx + 1, 0, ...blocks);
      return newBlocks;
    });
  }

  /**
   * Removes multiple blocks by their IDs.
   * @param blockIds - The IDs of the blocks to remove.
   */
  deleteBlocks(blockIds: string[]): void {
    const idSet = new Set(blockIds);
    this._blocks.update(blocks => blocks.filter(b => !idSet.has(b.id)));
  }

  /**
   * Atomically remove old blocks and insert new blocks at the target position.
   * This avoids the visual duplication that happens with separate delete + add calls.
   */
  replaceBlocks(oldBlockIds: string[], newBlocks: Block[], afterBlockId: string): void {
    const idSet = new Set(oldBlockIds);
    this._blocks.update(current => {
      const filtered = current.filter(b => !idSet.has(b.id));
      const idx = afterBlockId ? filtered.findIndex(b => b.id === afterBlockId) : -1;
      const result = [...filtered];
      result.splice(idx + 1, 0, ...newBlocks);
      return result;
    });
  }

  /**
   * Sets the set of currently selected block IDs.
   * @param ids - The set of block IDs to mark as selected.
   */
  setSelectedBlockIds(ids: Set<string>): void {
    this._selectedBlockIds.set(ids);
  }

  /**
   * Enables or disables block selection mode.
   * @param active - Whether block selection mode should be active.
   */
  setBlockSelectionMode(active: boolean): void {
    this._blockSelectionMode.set(active);
  }

  /**
   * Clears all selected blocks and exits block selection mode.
   */
  clearSelection(): void {
    this._selectedBlockIds.set(new Set());
    this._blockSelectionMode.set(false);
  }
}
