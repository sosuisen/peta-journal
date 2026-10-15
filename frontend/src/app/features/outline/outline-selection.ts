import { OutlineStateService } from '../../core/services/outline-state.service';
import { Item } from '../../core/models/item.model';
import { getSubtree } from './move-utils';
import { idsOfRangeWithSubtrees, selectionStartIndex } from './outline-tree';

/** What the keys pressed in selection mode do to the selected items. */
export interface SelectionCommands {
  undo(): void;
  redo(): void;
  deleteSelected(): void;
  moveSelected(direction: 'up' | 'down'): void;
  indentSelected(): void;
  outdentSelected(): void;
  copySelected(): void;
  cutSelected(): void;
}

type SelectionEventName = 'keydown' | 'click' | 'copy' | 'cut' | 'mouseup';

/**
 * Multi-item selection: which items are selected, how the selection grows with
 * Shift+Arrow or a mouse drag, and the document-level listeners that are active only
 * while a selection exists. The selected ids live in the outline state; this class
 * keeps the anchor and extent that the state does not need to know about.
 */
export class OutlineSelection {
  /** The item the selection started from. */
  private anchorItemId: string | null = null;
  /** The item the selection was last extended to. */
  private extentItemId: string | null = null;
  /** True while the mouse button is held after dragging out of an editor. */
  private isDragging = false;
  /** Document-level listeners that are active only in selection mode. */
  private readonly listeners: { [K in SelectionEventName]: (e: DocumentEventMap[K]) => void } = {
    keydown: e => this.onKeydown(e),
    click: () => this.exit(),
    copy: e => this.onCopy(e),
    cut: e => this.onCut(e),
    mouseup: () => this.onMouseUp(),
  };

  constructor(
    private readonly state: OutlineStateService,
    private readonly commands: SelectionCommands,
    private readonly doc: Document = document,
  ) {}

  /** Starts a keyboard selection from an editor: only the item itself, no subtree. */
  startFromEditor(item: Item): void {
    this.state.removeAllEditingItems();
    this.enter(item.id, new Set([item.id]));
  }

  /** Selects an item together with its subtree, e.g. the item the page was opened on. */
  selectSubtree(itemId: string): void {
    const subtreeIds = this.state.getChildren(itemId).map(b => b.id);
    this.enter(itemId, new Set([itemId, ...subtreeIds]));
  }

  /**
   * Starts a mouse-drag selection from an editor: the item and its children are selected,
   * and the selection follows the mouse until the button is released.
   */
  startDrag(item: Item): void {
    this.state.removeAllEditingItems();
    this.state.setItemSelectionMode(true);
    this.anchorItemId = item.id;
    this.isDragging = true;

    const group = this.state.getItemWithChildren(item.id);
    this.state.setSelectedItemIds(new Set(group.map(b => b.id)));

    // The click listener is added on mouse up, so the release does not end the selection
    this.listen('mouseup', 'keydown', 'copy', 'cut');
  }

  /** Extends a drag selection from the anchor to the item the mouse entered. */
  extendDragTo(item: Item): void {
    if (!this.isDragging) return;
    this.extentItemId = item.id;

    const items = this.state.visibleItems();
    const anchorIdx = items.findIndex(b => b.id === this.anchorItemId);
    const currentIdx = items.findIndex(b => b.id === item.id);
    if (anchorIdx < 0 || currentIdx < 0) return;

    this.selectRange(items, selectionStartIndex(items, anchorIdx, currentIdx), Math.max(anchorIdx, currentIdx));
  }

  /** Ends the selection and removes the document listeners. */
  exit(): void {
    this.state.clearSelection();
    this.anchorItemId = null;
    this.extentItemId = null;
    this.isDragging = false;
    this.stopListening();
  }

  /** Removes the document listeners without touching the state; for the owner's destroy. */
  dispose(): void {
    this.stopListening();
  }

  private enter(anchorItemId: string, selectedIds: Set<string>): void {
    this.state.setSelectedItemIds(selectedIds);
    this.state.setItemSelectionMode(true);
    this.anchorItemId = anchorItemId;
    this.extentItemId = anchorItemId;
    this.listen('keydown', 'click', 'copy', 'cut');
  }

  private listen(...names: SelectionEventName[]): void {
    for (const name of names) {
      this.doc.addEventListener(name, this.listeners[name] as EventListener);
    }
  }

  private stopListening(): void {
    for (const name of Object.keys(this.listeners) as SelectionEventName[]) {
      this.doc.removeEventListener(name, this.listeners[name] as EventListener);
    }
  }

  private extendUp(): void {
    const items = this.state.visibleItems();
    let extentIdx = items.findIndex(b => b.id === this.extentItemId);
    const anchorIdx = items.findIndex(b => b.id === this.anchorItemId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx <= 0) return;

    // When expanding upward, skip past items already in selection
    if (extentIdx <= anchorIdx) {
      const selected = this.state.selectedItemIds();
      while (extentIdx - 1 >= 0 && selected.has(items[extentIdx - 1].id)) {
        extentIdx--;
      }
      if (extentIdx <= 0) return;
    }

    this.extentItemId = items[extentIdx - 1].id;
    this.selectFromAnchorToExtent();
  }

  private extendDown(): void {
    const items = this.state.visibleItems();
    let extentIdx = items.findIndex(b => b.id === this.extentItemId);
    const anchorIdx = items.findIndex(b => b.id === this.anchorItemId);
    if (extentIdx < 0 || anchorIdx < 0 || extentIdx >= items.length - 1) return;

    if (extentIdx >= anchorIdx) {
      // When expanding downward, skip past items already in selection
      const selected = this.state.selectedItemIds();
      while (extentIdx + 1 < items.length && selected.has(items[extentIdx + 1].id)) {
        extentIdx++;
      }
      if (extentIdx >= items.length - 1) return;
    } else {
      // When shrinking from above, the extent is an ancestor that was pulled in with its
      // subtree; step past the subtree, or the next selection would pull it in again
      const subtreeEnd = extentIdx + getSubtree(items, items[extentIdx].id).length - 1;
      extentIdx = Math.min(subtreeEnd, anchorIdx - 1);
    }

    this.extentItemId = items[extentIdx + 1].id;
    this.selectFromAnchorToExtent();
  }

  private selectFromAnchorToExtent(): void {
    const items = this.state.visibleItems();
    const anchorIdx = items.findIndex(b => b.id === this.anchorItemId);
    const extentIdx = items.findIndex(b => b.id === this.extentItemId);
    if (anchorIdx < 0 || extentIdx < 0) return;

    const start = selectionStartIndex(items, anchorIdx, extentIdx);
    // When selecting upward into a subtree, the extent moves to the ancestor that was
    // pulled in, so the next Shift+Up continues from the correct position
    if (extentIdx < anchorIdx && start < extentIdx) {
      this.extentItemId = items[start].id;
    }
    this.selectRange(items, start, Math.max(anchorIdx, extentIdx));
  }

  /** Selects the items from start to end (inclusive) together with their subtrees. */
  private selectRange(items: Item[], start: number, end: number): void {
    this.state.setSelectedItemIds(idsOfRangeWithSubtrees(items, this.state.items(), start, end));
  }

  private onMouseUp(): void {
    this.isDragging = false;
    this.doc.removeEventListener('mouseup', this.listeners.mouseup);

    // Add click listener to detect clicks outside selection
    // Use setTimeout to avoid the current mouseup from triggering it
    setTimeout(() => this.listen('click'), 0);
  }

  private onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      this.exit();
      return;
    }

    // Ctrl+Z / Ctrl+Shift+Z in selection mode
    if ((e.key === 'z' || e.key === 'Z') && e.ctrlKey && !e.altKey) {
      e.preventDefault();
      if (e.shiftKey) {
        this.commands.redo();
      } else {
        this.commands.undo();
      }
      return;
    }

    if (e.key === 'Delete' || e.key === 'Backspace') {
      e.preventDefault();
      this.commands.deleteSelected();
      return;
    }

    if (e.shiftKey && e.altKey) {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.commands.moveSelected('up');
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.commands.moveSelected('down');
        return;
      }
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        this.commands.outdentSelected();
        return;
      }
      if (e.key === 'ArrowRight') {
        e.preventDefault();
        this.commands.indentSelected();
        return;
      }
    }

    if (e.shiftKey && !e.altKey && !e.ctrlKey) {
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        this.extendUp();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.extendDown();
        return;
      }
    }

    if (e.key === 'Tab') {
      e.preventDefault();
      if (e.shiftKey) {
        this.commands.outdentSelected();
      } else {
        this.commands.indentSelected();
      }
      return;
    }

    if (e.ctrlKey && e.key === 'c') {
      e.preventDefault();
      this.commands.copySelected();
      return;
    }

    if (e.ctrlKey && e.key === 'x') {
      e.preventDefault();
      this.commands.cutSelected();
      return;
    }
  }

  private onCopy(e: ClipboardEvent): void {
    if (!this.state.itemSelectionMode()) return;
    e.preventDefault();
    this.commands.copySelected();
  }

  private onCut(e: ClipboardEvent): void {
    if (!this.state.itemSelectionMode()) return;
    e.preventDefault();
    this.commands.cutSelected();
  }
}
