import { Injectable, signal } from '@angular/core';
import { Item } from '../models/item.model';

export type UndoOp =
  | { type: 'update'; before: Item; after: Item }
  | { type: 'create'; item: Item }
  | { type: 'delete'; item: Item };

export interface CursorState {
  itemId: string;
  position: number;
}

export interface UndoEntry {
  ops: UndoOp[];
  cursorBefore: CursorState | null;
  cursorAfter: CursorState | null;
  timestamp: number;
  label: string;
}

const MAX_STACK_SIZE = 100;
const BODY_GROUP_INTERVAL_MS = 500;

@Injectable({ providedIn: 'root' })
export class UndoService {
  private undoStack: UndoEntry[] = [];
  private redoStack: UndoEntry[] = [];
  private pendingOps: UndoOp[] = [];
  private pendingLabel = '';
  private pendingCursorBefore: CursorState | null = null;
  private pending = false;

  readonly canUndo = signal(false);
  readonly canRedo = signal(false);

  begin(label: string, cursorBefore?: CursorState | null): void {
    this.pendingOps = [];
    this.pendingLabel = label;
    this.pendingCursorBefore = cursorBefore ?? null;
    this.pending = true;
  }

  pushUpdate(before: Item, after: Item): void {
    this.pendingOps.push({ type: 'update', before: { ...before }, after: { ...after } });
  }

  pushCreate(item: Item): void {
    this.pendingOps.push({ type: 'create', item: { ...item } });
  }

  pushDelete(item: Item): void {
    this.pendingOps.push({ type: 'delete', item: { ...item } });
  }

  commit(cursorAfter?: CursorState | null): void {
    if (!this.pending || this.pendingOps.length === 0) {
      this.pending = false;
      return;
    }

    const now = Date.now();

    // Body-edit grouping: merge with previous entry if same label 'body' and same itemId within interval
    if (this.pendingLabel === 'body' && this.undoStack.length > 0) {
      const top = this.undoStack[this.undoStack.length - 1];
      if (
        top.label === 'body' &&
        now - top.timestamp < BODY_GROUP_INTERVAL_MS &&
        top.ops.length === 1 &&
        this.pendingOps.length === 1 &&
        top.ops[0].type === 'update' &&
        this.pendingOps[0].type === 'update' &&
        top.ops[0].after.id === (this.pendingOps[0] as { type: 'update'; before: Item; after: Item }).before.id
      ) {
        // Merge: keep old before, update after and timestamp
        const topOp = top.ops[0] as { type: 'update'; before: Item; after: Item };
        const newOp = this.pendingOps[0] as { type: 'update'; before: Item; after: Item };
        topOp.after = newOp.after;
        top.cursorAfter = cursorAfter ?? null;
        top.timestamp = now;
        this.pending = false;
        this.redoStack = [];
        this.updateSignals();
        return;
      }
    }

    const entry: UndoEntry = {
      ops: this.pendingOps,
      cursorBefore: this.pendingCursorBefore,
      cursorAfter: cursorAfter ?? null,
      timestamp: now,
      label: this.pendingLabel,
    };

    this.undoStack.push(entry);
    if (this.undoStack.length > MAX_STACK_SIZE) {
      this.undoStack.shift();
    }
    this.redoStack = [];
    this.pending = false;
    this.updateSignals();
  }

  abort(): void {
    this.pendingOps = [];
    this.pending = false;
  }

  isPending(): boolean {
    return this.pending;
  }

  popUndo(): UndoEntry | null {
    const entry = this.undoStack.pop() ?? null;
    this.updateSignals();
    return entry;
  }

  popRedo(): UndoEntry | null {
    const entry = this.redoStack.pop() ?? null;
    this.updateSignals();
    return entry;
  }

  pushRedo(entry: UndoEntry): void {
    this.redoStack.push(entry);
    if (this.redoStack.length > MAX_STACK_SIZE) {
      this.redoStack.shift();
    }
    this.updateSignals();
  }

  pushUndo(entry: UndoEntry): void {
    this.undoStack.push(entry);
    if (this.undoStack.length > MAX_STACK_SIZE) {
      this.undoStack.shift();
    }
    this.updateSignals();
  }

  clear(): void {
    this.undoStack = [];
    this.redoStack = [];
    this.pendingOps = [];
    this.pending = false;
    this.updateSignals();
  }

  private updateSignals(): void {
    this.canUndo.set(this.undoStack.length > 0);
    this.canRedo.set(this.redoStack.length > 0);
  }
}
