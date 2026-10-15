import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Item } from '../../core/models/item.model';
import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { UndoEditor, UndoReplayService } from './undo-replay.service';

function b(id: string, level: number, extra: Partial<Item> = {}): Item {
  return {
    id,
    pageId: 'page1',
    dirPath: '',
    name: '',
    type: 'doc',
    createdDate: 0,
    modifiedDate: 0,
    idx: id,
    level,
    body: 'body of ' + id,
    ast: '',
    collapsed: false,
    types: [],
    properties: {},
    ...extra,
  };
}

/** Records the calls and answers like the server would: an update echoes the fields, a create gets a new id. */
function fakeApi() {
  return {
    updateItem: vi.fn(async (id: string, updates: Partial<Item>) => ({ ...b(id, 1), ...updates })),
    createItem: vi.fn(async (pageId: string, _pos: string, level: number, body: string) =>
      b('server-' + body, level, { pageId, body })),
    deleteItem: vi.fn(async () => {}),
  };
}

/** Waits for the background server sync, which the replay does not await. */
const settled = () => new Promise(resolve => setTimeout(resolve, 0));

describe('UndoReplayService', () => {
  let state: OutlineStateService;
  let undo: UndoService;
  let api: ReturnType<typeof fakeApi>;
  let editor: UndoEditor;
  let service: UndoReplayService;

  beforeEach(() => {
    state = new OutlineStateService();
    undo = new UndoService();
    api = fakeApi();
    editor = { discardPendingEdit: vi.fn(), focusEditor: vi.fn() };
    service = new UndoReplayService(state, api as unknown as ApiService, undo);
  });

  describe('body update', () => {
    const before = b('a', 1, { body: 'old' });
    const after = { ...before, body: 'new' };

    beforeEach(() => {
      state.setItems([after]);
      undo.begin('body');
      undo.pushUpdate(before, after);
      undo.commit();
    });

    it('undo puts the old body back, saves it, and moves the entry to the redo stack', async () => {
      expect(await service.replay('undo', editor)).toBe(true);
      await settled();

      expect(state.getItemById('a')?.body).toBe('old');
      expect(api.updateItem).toHaveBeenCalledWith('a', { body: 'old', level: 1, collapsed: false });
      expect(undo.canUndo()).toBe(false);
      expect(undo.canRedo()).toBe(true);
    });

    it('redo after undo brings the new body back', async () => {
      await service.replay('undo', editor);
      await service.replay('redo', editor);
      await settled();

      expect(state.getItemById('a')?.body).toBe('new');
      expect(api.updateItem).toHaveBeenLastCalledWith('a', { body: 'new', level: 1, collapsed: false });
      expect(undo.canUndo()).toBe(true);
    });

    it('keeps editing the updated item, with the cursor at the end of its body', async () => {
      await service.replay('undo', editor);

      expect(editor.discardPendingEdit).toHaveBeenCalledOnce();
      expect(state.focusedItemId()).toBe('a');
      expect(state.pendingCursorPosition()).toBe('old'.length);
      expect(state.editingItemIds().has('a')).toBe(true);
      expect(editor.focusEditor).toHaveBeenCalledWith('a');
    });
  });

  describe('create and delete', () => {
    it('undo of a create removes the item here and on the server', async () => {
      const created = b('n', 1, { idx: 'b' });
      state.setItems([b('a', 1, { idx: 'a' }), created]);
      undo.begin('enter');
      undo.pushCreate(created);
      undo.commit();

      await service.replay('undo', editor);
      await settled();

      expect(state.items().map(i => i.id)).toEqual(['a']);
      expect(api.deleteItem).toHaveBeenCalledWith('n');
    });

    it('redo of a create puts the item back by its index and takes the id the server gives', async () => {
      const created = b('n', 1, { idx: 'b', body: 'again' });
      state.setItems([b('a', 1, { idx: 'a' }), created, b('c', 1, { idx: 'c' })]);
      undo.begin('enter');
      undo.pushCreate(created);
      undo.commit();

      await service.replay('undo', editor);
      await service.replay('redo', editor);
      await settled();

      expect(state.items().map(i => i.id)).toEqual(['a', 'server-again', 'c']);
      expect(api.createItem).toHaveBeenCalledWith('page1', 'after:a', 1, 'again');
    });

    it('undo of a delete recreates the item where it was', async () => {
      const deleted = b('d', 1, { idx: 'b', body: 'back' });
      state.setItems([b('a', 1, { idx: 'a' }), b('c', 1, { idx: 'c' })]);
      undo.begin('delete-selection');
      undo.pushDelete(deleted);
      undo.commit();

      await service.replay('undo', editor);
      await settled();

      expect(state.items().map(i => i.id)).toEqual(['a', 'server-back', 'c']);
      expect(api.createItem).toHaveBeenCalledWith('page1', 'after:a', 1, 'back');
    });
  });

  describe('cursor', () => {
    it('uses the recorded cursor when the entry has one', async () => {
      const item = b('a', 1);
      state.setItems([item]);
      undo.begin('enter', { itemId: 'a', position: 3 });
      undo.pushCreate(b('n', 1));
      undo.commit({ itemId: 'n', position: 0 });

      await service.replay('undo', editor);

      expect(state.focusedItemId()).toBe('a');
      expect(state.pendingCursorPosition()).toBe(3);
    });

    it('closes every editor when neither a cursor nor an update is recorded', async () => {
      state.setItems([b('a', 1)]);
      state.addEditingItem('a');
      undo.begin('delete-selection');
      undo.pushDelete(b('d', 1));
      undo.commit();

      await service.replay('undo', editor);

      expect(state.editingItemIds().size).toBe(0);
      expect(editor.focusEditor).not.toHaveBeenCalled();
    });
  });

  describe('nothing to do', () => {
    it('returns false on an empty stack and moves no cursor', async () => {
      expect(await service.replay('undo', editor)).toBe(false);
      expect(editor.focusEditor).not.toHaveBeenCalled();
    });

    it('does nothing while an entry is still being recorded', async () => {
      undo.begin('body');
      expect(await service.replay('undo', editor)).toBe(false);
    });
  });

  it('a collapse change also flips the collapsed set', async () => {
    const before = b('a', 1, { collapsed: false });
    state.setItems([{ ...before, collapsed: true }]);
    undo.begin('collapse');
    undo.pushUpdate(before, { ...before, collapsed: true });
    undo.commit();

    await service.replay('undo', editor);

    expect(state.isCollapsed('a')).toBe(false);
    expect(state.getItemById('a')?.collapsed).toBe(false);
  });
});
