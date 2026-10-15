import { beforeEach, describe, expect, it } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { ConfigService } from '../../core/services/config.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { LevelShiftService } from './level-shift.service';
import { fakeApi, fakeView, item } from './outline-test-support';

describe('LevelShiftService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi>;
  let undo: UndoService;
  let logicalOutdent: boolean;
  let service: LevelShiftService;
  let view: ReturnType<typeof fakeView>;

  const levels = () => state.items().map(i => `${i.id}:${i.level}`);

  beforeEach(() => {
    state = new OutlineStateService();
    api = fakeApi();
    undo = new UndoService();
    logicalOutdent = false;
    const config = { logicalOutdent: () => logicalOutdent } as unknown as ConfigService;
    service = new LevelShiftService(state, api as unknown as ApiService, undo, config);
    view = fakeView();
  });

  describe('indent', () => {
    it('moves the item with its children one level deeper and saves one relative shift', async () => {
      // * a (1)
      // * b (1)    ← indent
      //     * c (2)
      state.setItems([item('a', 1), item('b', 1), item('c', 2)]);

      await service.indent(state.getItemById('b')!, view);

      expect(levels()).toEqual(['a:1', 'b:2', 'c:3']);
      expect(api.shiftItemLevels).toHaveBeenCalledWith(['b', 'c'], 1);
      expect(undo.canUndo()).toBe(true);
    });

    it('does nothing for the first item', async () => {
      state.setItems([item('a', 1), item('b', 1)]);

      await service.indent(state.getItemById('a')!, view);

      expect(levels()).toEqual(['a:1', 'b:1']);
      expect(api.shiftItemLevels).not.toHaveBeenCalled();
    });

    it('reloads the pages when the save fails, so the screen matches the server', async () => {
      state.setItems([item('a', 1), item('b', 1)]);
      api.shiftItemLevels.mockRejectedValueOnce(new Error('offline'));

      await service.indent(state.getItemById('b')!, view);

      expect(view.reloadPageFromFile).toHaveBeenCalledWith('page1');
    });
  });

  describe('outdent', () => {
    // * p (1)
    //     * a (2)
    //     * b (2)    ← outdent
    //         * c (3)
    //     * d (2)
    beforeEach(() => {
      state.setItems([item('p', 1), item('a', 2), item('b', 2), item('c', 3), item('d', 2)]);
    });

    it('in place: the group goes up one level and keeps its place', async () => {
      await service.outdent(state.getItemById('b')!, view);

      expect(levels()).toEqual(['p:1', 'a:2', 'b:1', 'c:2', 'd:2']);
      expect(api.shiftItemLevels).toHaveBeenCalledWith(['b', 'c'], -1);
      expect(api.moveItems).not.toHaveBeenCalled();
    });

    it('logical: the group moves after the parent\'s last child so d stays under p', async () => {
      logicalOutdent = true;

      await service.outdent(state.getItemById('b')!, view);

      expect(api.shiftItemLevels).toHaveBeenCalledWith(['b', 'c'], -1);
      expect(api.moveItems).toHaveBeenCalledWith(['b', 'c'], 'page1', 'after:d');
      expect(state.items().map(i => i.id)).toEqual(['p', 'a', 'd', 'b', 'c']);
    });

    it('logical: an only child needs no move', async () => {
      // * p (1)
      //     * only (2)    ← outdent
      // * q (1)
      logicalOutdent = true;
      state.setItems([item('p', 1), item('only', 2), item('q', 1)]);

      await service.outdent(state.getItemById('only')!, view);

      expect(api.moveItems).not.toHaveBeenCalled();
      expect(levels()).toEqual(['p:1', 'only:1', 'q:1']);
    });

    it('does nothing at the top level', async () => {
      await service.outdent(state.getItemById('p')!, view);

      expect(api.shiftItemLevels).not.toHaveBeenCalled();
    });
  });

  describe('inside a transclusion', () => {
    // own:     root (1, transcludes s)
    // source:  s (1, page src) > s1 (2) > s2 (3)
    // visible: root (1), s1 (2), s2 (3)
    beforeEach(() => {
      state.setItems([item('root', 1, { transcludeItemId: 's' })]);
      state.setTranscludedTrees({ s: [item('s', 1, { pageId: 'src' }), item('s1', 2, { pageId: 'src' }), item('s2', 3, { pageId: 'src' })] });
    });

    it('a descendant cannot be outdented onto the root\'s level', async () => {
      const s1 = state.visibleItems().find(i => i.id === 's1')!;

      await service.outdent(s1, view);

      expect(api.shiftItemLevels).not.toHaveBeenCalled();
    });

    it('a deeper descendant is outdented and stored with the tree\'s level offset', async () => {
      const s2 = state.visibleItems().find(i => i.id === 's2')!;

      await service.outdent(s2, view);

      expect(api.shiftItemLevels).toHaveBeenCalledWith(['s2'], -1);
      expect(state.getItemById('s2')?.level).toBe(2);
    });
  });
});
