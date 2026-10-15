import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { item } from '../testing/item-fixture';
import { UndoService } from './undo.service';

describe('UndoService', () => {
  let undo: UndoService;

  const a = item('a', 1, { body: 'one' });
  const a2 = { ...a, body: 'two' };
  const a3 = { ...a, body: 'three' };

  beforeEach(() => {
    vi.useFakeTimers();
    undo = new UndoService();
  });

  afterEach(() => vi.useRealTimers());

  describe('recording an entry', () => {
    it('begin, push and commit make one entry with its label and cursors', () => {
      undo.begin('enter', { itemId: 'a', position: 3 });
      undo.pushUpdate(a, a2);
      undo.pushCreate(item('n', 1));
      undo.commit({ itemId: 'n', position: 0 });

      expect(undo.canUndo()).toBe(true);
      expect(undo.popUndo()).toMatchObject({
        label: 'enter',
        cursorBefore: { itemId: 'a', position: 3 },
        cursorAfter: { itemId: 'n', position: 0 },
        ops: [{ type: 'update', before: a, after: a2 }, { type: 'create', item: { id: 'n' } }],
      });
      expect(undo.canUndo()).toBe(false);
    });

    it('stores copies, so later changes to the items do not leak into the entry', () => {
      const before = { ...a };
      undo.begin('body');
      undo.pushUpdate(before, a2);
      before.body = 'mutated';
      undo.commit();

      expect(undo.popUndo()?.ops[0]).toMatchObject({ before: { body: 'one' } });
    });

    it('a commit without ops or without begin records nothing', () => {
      undo.begin('empty');
      undo.commit();
      undo.pushDelete(a);
      undo.commit();

      expect(undo.canUndo()).toBe(false);
      expect(undo.isPending()).toBe(false);
    });

    it('abort drops what was pushed since begin', () => {
      undo.begin('x');
      undo.pushDelete(a);
      expect(undo.isPending()).toBe(true);

      undo.abort();
      undo.commit();

      expect(undo.canUndo()).toBe(false);
    });
  });

  describe('grouping body edits', () => {
    const commitBody = (before: typeof a, after: typeof a, cursor = 0) => {
      undo.begin('body');
      undo.pushUpdate(before, after);
      undo.commit({ itemId: after.id, position: cursor });
    };

    it('merges quick edits of the same item into one entry from the first before to the last after', () => {
      commitBody(a, a2, 3);
      vi.advanceTimersByTime(100);
      commitBody(a2, a3, 5);

      const entry = undo.popUndo();
      expect(entry?.ops).toEqual([{ type: 'update', before: a, after: a3 }]);
      expect(entry?.cursorAfter).toEqual({ itemId: 'a', position: 5 });
      expect(undo.canUndo()).toBe(false);
    });

    it('starts a new entry after a pause', () => {
      commitBody(a, a2);
      vi.advanceTimersByTime(500);
      commitBody(a2, a3);

      undo.popUndo();
      expect(undo.canUndo()).toBe(true);
    });

    it('does not merge edits of different items or with another label', () => {
      const b = item('b', 1);
      commitBody(a, a2);
      commitBody(b, { ...b, body: 'x' });
      undo.begin('enter');
      undo.pushUpdate(a2, a3);
      undo.commit();

      undo.popUndo();
      undo.popUndo();
      expect(undo.canUndo()).toBe(true);
    });
  });

  describe('undo and redo stacks', () => {
    it('an entry moves between the stacks and a new commit clears the redo stack', () => {
      undo.begin('x');
      undo.pushDelete(a);
      undo.commit();

      const entry = undo.popUndo()!;
      undo.pushRedo(entry);
      expect(undo.canUndo()).toBe(false);
      expect(undo.canRedo()).toBe(true);

      expect(undo.popRedo()).toBe(entry);
      undo.pushUndo(entry);
      expect(undo.canUndo()).toBe(true);

      undo.begin('y');
      undo.pushDelete(a);
      undo.commit();
      expect(undo.canRedo()).toBe(false);
    });

    it('keeps at most 100 entries and drops the oldest', () => {
      for (let i = 0; i < 101; i++) {
        undo.begin('n' + i);
        undo.pushDelete(a);
        undo.commit();
      }

      const labels: string[] = [];
      for (let e = undo.popUndo(); e; e = undo.popUndo()) labels.push(e.label);
      expect(labels.length).toBe(100);
      expect(labels[labels.length - 1]).toBe('n1');
    });

    it('clear empties both stacks', () => {
      undo.begin('x');
      undo.pushDelete(a);
      undo.commit();
      undo.pushRedo(undo.popUndo()!);

      undo.clear();

      expect(undo.canUndo()).toBe(false);
      expect(undo.canRedo()).toBe(false);
    });
  });
});
