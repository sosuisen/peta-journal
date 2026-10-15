import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { ItemEditService } from './item-edit.service';
import { ItemMergeService } from './item-merge.service';
import { fakeApi, fakeView, item } from './outline-test-support';

describe('ItemMergeService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi>;
  let undo: UndoService;
  let view: ReturnType<typeof fakeView>;
  let service: ItemMergeService;

  const bodies = () => state.items().map(i => `${i.id}=${i.body}`);

  beforeEach(() => {
    state = new OutlineStateService();
    api = fakeApi();
    undo = new UndoService();
    const autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    const edits = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
    service = new ItemMergeService(state, api as unknown as ApiService, undo, edits);
    view = fakeView();
    state.setItems([item('a', 1, { body: 'hello' }), item('b', 1, { body: ' world' }), item('c', 1)]);
  });

  describe('Backspace at the start', () => {
    it('appends the text to the item above, deletes the item and keeps editing above', async () => {
      await service.mergeIntoPrevious(state.getItemById('b')!, view);

      expect(bodies()).toEqual(['a=hello world', 'c=body of c']);
      expect(api.updateItem).toHaveBeenCalledWith('a', { body: 'hello world' });
      expect(api.deleteItem).toHaveBeenCalledWith('b');
      expect(state.editingItemIds().has('a')).toBe(true);
      expect(view.focusEditor).toHaveBeenCalledWith('a');
    });

    it('records one undo entry that puts the cursor at the join', async () => {
      await service.mergeIntoPrevious(state.getItemById('b')!, view);

      const entry = undo.popUndo();
      expect(entry?.ops.map(op => op.type)).toEqual(['update', 'delete']);
      expect(entry?.cursorBefore).toEqual({ itemId: 'b', position: 0 });
      expect(entry?.cursorAfter).toEqual({ itemId: 'a', position: 'hello'.length });
    });

    it('does nothing for the first item', async () => {
      await service.mergeIntoPrevious(state.getItemById('a')!, view);

      expect(bodies()).toHaveLength(3);
      expect(api.deleteItem).not.toHaveBeenCalled();
    });

    it('does not merge a transclusion item away', async () => {
      state.setItems([item('a', 1), item('t', 1, { transcludeItemId: 's' })]);

      await service.mergeIntoPrevious(state.getItemById('t')!, view);

      expect(api.deleteItem).not.toHaveBeenCalled();
    });

    it('does not merge the first item of a journal day into the day above', async () => {
      state.setItems([
        item('d2', 1, { type: 'journal', name: '2026-03-02', pageId: 'p2' }),
        item('d1', 1, { type: 'journal', name: '2026-03-01', pageId: 'p1' }),
      ]);

      await service.mergeIntoPrevious(state.getItemById('d1')!, view);

      expect(api.deleteItem).not.toHaveBeenCalled();
    });
  });

  describe('Delete at the end', () => {
    it('appends the item below and deletes it', async () => {
      await service.mergeNextInto(state.getItemById('a')!);

      expect(bodies()).toEqual(['a=hello world', 'c=body of c']);
      expect(api.updateItem).toHaveBeenCalledWith('a', { body: 'hello world' });
      expect(api.deleteItem).toHaveBeenCalledWith('b');
      expect(undo.popUndo()?.cursorAfter).toEqual({ itemId: 'a', position: 'hello'.length });
    });

    it('does nothing for the last item', async () => {
      await service.mergeNextInto(state.getItemById('c')!);

      expect(api.deleteItem).not.toHaveBeenCalled();
    });

    it('does not pull a transclusion item in', async () => {
      state.setItems([item('a', 1), item('t', 1, { transcludeItemId: 's' })]);

      await service.mergeNextInto(state.getItemById('a')!);

      expect(api.deleteItem).not.toHaveBeenCalled();
    });

    it('does not pull the next journal day in', async () => {
      state.setItems([
        item('d2', 1, { type: 'journal', name: '2026-03-02', pageId: 'p2' }),
        item('d1', 1, { type: 'journal', name: '2026-03-01', pageId: 'p1' }),
      ]);

      await service.mergeNextInto(state.getItemById('d2')!);

      expect(api.deleteItem).not.toHaveBeenCalled();
    });
  });
});
