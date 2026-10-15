import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { ItemEditService } from './item-edit.service';
import { ItemMoveService } from './item-move.service';
import { JournalDayMoveService } from './journal-day-move.service';
import { fakeApi, fakeView, item } from './outline-test-support';

describe('ItemMoveService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi>;
  let undo: UndoService;
  let journalMoves: { moveAcrossDays: ReturnType<typeof vi.fn> };
  let view: ReturnType<typeof fakeView>;
  let service: ItemMoveService;

  const ids = () => state.items().map(i => i.id);

  beforeEach(() => {
    state = new OutlineStateService();
    api = fakeApi();
    undo = new UndoService();
    const autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    const edits = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
    journalMoves = { moveAcrossDays: vi.fn(async () => null) };
    service = new ItemMoveService(state, api as unknown as ApiService, undo, edits,
      journalMoves as unknown as JournalDayMoveService);
    view = fakeView();
  });

  describe('on a page', () => {
    // * a (1)
    // * b (1)
    //     * b1 (2)
    // * c (1)
    beforeEach(() => state.setItems([item('a', 1), item('b', 1), item('b1', 2), item('c', 1)]));

    it('moves the item with its children past the sibling above and keeps its editor', async () => {
      const result = await service.move(state.getItemById('b')!, 'up', view);

      expect(result).toBeNull();
      expect(api.moveItems).toHaveBeenCalledWith(['b', 'b1'], 'page1', 'first');
      expect(ids()).toEqual(['b', 'b1', 'a', 'c']);
      expect(view.focusEditor).toHaveBeenCalledWith('b');
      expect(undo.popUndo()?.label).toBe('move');
    });

    it('moves the item past the sibling below', async () => {
      await service.move(state.getItemById('b')!, 'down', view);

      expect(api.moveItems).toHaveBeenCalledWith(['b', 'b1'], 'page1', 'after:c');
      expect(ids()).toEqual(['a', 'c', 'b', 'b1']);
    });

    it('does nothing at the top or bottom', async () => {
      await service.move(state.getItemById('a')!, 'up', view);
      await service.move(state.getItemById('c')!, 'down', view);

      expect(api.moveItems).not.toHaveBeenCalled();
    });
  });

  describe('in a journal', () => {
    const day = (id: string, level: number, date: string) =>
      item(id, level, { pageId: 'page-' + date, type: 'journal', name: date });

    // 2026-03-02: n1, n2
    // 2026-03-01: t1, t2
    beforeEach(() => state.setItems([day('n1', 1, '2026-03-02'), day('n2', 1, '2026-03-02'), day('t1', 1, '2026-03-01'), day('t2', 1, '2026-03-01')]));

    it('asks the journal mover first and returns what it did', async () => {
      journalMoves.moveAcrossDays.mockResolvedValueOnce({ targetDate: '2026-03-02', navigate: true, itemId: 't1' });

      const result = await service.move(state.getItemById('t1')!, 'up', view);

      expect(journalMoves.moveAcrossDays).toHaveBeenCalledWith(state.getItemById('t1'), 'up', 'page1');
      expect(result).toEqual({ targetDate: '2026-03-02', navigate: true, itemId: 't1' });
      expect(api.moveItems).not.toHaveBeenCalled();
    });

    it('otherwise moves within the item\'s own page, with "first" mapped into the full list', async () => {
      await service.move(state.getItemById('t2')!, 'up', view);

      expect(api.moveItems).toHaveBeenCalledWith(['t2'], 'page-2026-03-01', 'first');
      expect(ids()).toEqual(['n1', 'n2', 't2', 't1']);
    });
  });

  describe('inside a transclusion', () => {
    // own: root (1, transcludes s); source: s (1) > s1 (2), s2 (2), s3 (2)
    beforeEach(() => {
      state.setItems([item('root', 1, { transcludeItemId: 's' })]);
      state.setTranscludedTrees({ s: [item('s', 1, { pageId: 'src' }), item('s1', 2, { pageId: 'src' }), item('s2', 2, { pageId: 'src' }), item('s3', 2, { pageId: 'src' })] });
    });

    it('moves a descendant within the source tree and reloads the page', async () => {
      const s3 = state.visibleItems().find(i => i.id === 's3')!;

      await service.move(s3, 'up', view);

      expect(api.moveItems).toHaveBeenCalledWith(['s3'], 'src', 'after:s1');
      expect(view.reloadPage).toHaveBeenCalled();
      expect(view.focusEditor).toHaveBeenCalledWith('s3');
    });

    it('moving the second child above the first keeps it under the source root', async () => {
      const s2 = state.visibleItems().find(i => i.id === 's2')!;

      await service.move(s2, 'up', view);

      expect(api.moveItems).toHaveBeenCalledWith(['s2'], 'src', 'after:s');
    });
  });

  describe('moveSelection', () => {
    // * a (1)
    // * b (1)
    //     * b1 (2)
    // * c (1)
    beforeEach(() => state.setItems([item('a', 1), item('b', 1), item('b1', 2), item('c', 1)]));

    it('moves the selected subtrees as one group past the sibling of the edge root', async () => {
      state.setSelectedItemIds(new Set(['b', 'b1', 'c']));

      await service.moveSelection('up');

      expect(api.moveItems).toHaveBeenCalledWith(['b', 'b1', 'c'], 'page1', 'first');
      expect(ids()).toEqual(['b', 'b1', 'c', 'a']);
      expect(undo.popUndo()?.ops).toHaveLength(3);
    });

    it('does nothing without a selection or at the edge', async () => {
      await service.moveSelection('up');
      state.setSelectedItemIds(new Set(['c']));
      await service.moveSelection('down');

      expect(api.moveItems).not.toHaveBeenCalled();
    });
  });
});
