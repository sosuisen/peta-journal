import { Injectable, signal, computed } from '@angular/core';
import { Block } from '../models/block.model';

@Injectable({ providedIn: 'root' })
export class OutlineStateService {
  private readonly _blocks = signal<Block[]>([]);
  private readonly _editingBlockId = signal<string | null>(null);
  private readonly _selectedBlockIds = signal<Set<string>>(new Set());
  private readonly _collapsedBlockIds = signal<Set<string>>(new Set());
  private readonly _blockSelectionMode = signal<boolean>(false);
  private readonly _pendingCursorPosition = signal<number | null>(null);
  readonly blocks = this._blocks.asReadonly();
  readonly editingBlockId = this._editingBlockId.asReadonly();
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

  setBlocks(blocks: Block[]): void {
    this._blocks.set(blocks);
    const collapsed = new Set<string>();
    for (const b of blocks) {
      if (b.collapsed) collapsed.add(b.id);
    }
    this._collapsedBlockIds.set(collapsed);
  }

  addBlock(block: Block, afterBlockId: string): void {
    this._blocks.update(blocks => {
      const idx = afterBlockId ? blocks.findIndex(b => b.id === afterBlockId) : -1;
      const newBlocks = [...blocks];
      newBlocks.splice(idx + 1, 0, block);
      return newBlocks;
    });
  }

  updateBlock(blockId: string, updates: Partial<Block>): void {
    this._blocks.update(blocks =>
      blocks.map(b => b.id === blockId ? { ...b, ...updates } : b)
    );
  }

  deleteBlock(blockId: string): void {
    this._blocks.update(blocks => blocks.filter(b => b.id !== blockId));
  }

  setEditingBlock(blockId: string | null, cursorPosition: number | null = null): void {
    this._pendingCursorPosition.set(cursorPosition);
    this._editingBlockId.set(blockId);
  }


  consumePendingCursorPosition(): number | null {
    const pos = this._pendingCursorPosition();
    this._pendingCursorPosition.set(null);
    return pos;
  }

  toggleCollapsed(blockId: string): void {
    this._collapsedBlockIds.update(set => {
      const next = new Set(set);
      if (next.has(blockId)) next.delete(blockId);
      else next.add(blockId);
      return next;
    });
  }

  getBlockById(blockId: string): Block | undefined {
    return this._blocks().find(b => b.id === blockId);
  }

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

  addBlocks(blocks: Block[], afterBlockId: string): void {
    this._blocks.update(current => {
      const idx = afterBlockId ? current.findIndex(b => b.id === afterBlockId) : -1;
      const newBlocks = [...current];
      newBlocks.splice(idx + 1, 0, ...blocks);
      return newBlocks;
    });
  }

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

  setSelectedBlockIds(ids: Set<string>): void {
    this._selectedBlockIds.set(ids);
  }

  setBlockSelectionMode(active: boolean): void {
    this._blockSelectionMode.set(active);
  }

  clearSelection(): void {
    this._selectedBlockIds.set(new Set());
    this._blockSelectionMode.set(false);
  }
}
