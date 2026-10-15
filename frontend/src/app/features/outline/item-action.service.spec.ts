import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { ClipboardService } from '../../core/services/clipboard.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemActionService, zoomOf } from './item-action.service';
import { ItemEditService } from './item-edit.service';
import { fakeApi, fakeView, item } from './outline-test-support';

describe('ItemActionService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi> & {
    cycleTask: ReturnType<typeof vi.fn>;
    upsertItemProperty: ReturnType<typeof vi.fn>;
    setTransclusion: ReturnType<typeof vi.fn>;
    removeTransclusion: ReturnType<typeof vi.fn>;
    exportTreeMarkdown: ReturnType<typeof vi.fn>;
  };
  let undo: UndoService;
  let clipboard: ClipboardService;
  let view: ReturnType<typeof fakeView>;
  let service: ItemActionService;

  beforeEach(() => {
    state = new OutlineStateService();
    api = {
      ...fakeApi(),
      cycleTask: vi.fn(async (id: string) => item(id, 1, { types: ['task'], properties: { status: 'todo' } })),
      upsertItemProperty: vi.fn(async (id: string, key: string, _type: string, value: string) =>
        item(id, 1, { types: ['task'], properties: { status: 'todo', [key]: value } })),
      setTransclusion: vi.fn(async (id: string, sourceId: string) => item(id, 1, { transcludeItemId: sourceId })),
      removeTransclusion: vi.fn(async (id: string) => item(id, 1, { transcludeItemId: null })),
      exportTreeMarkdown: vi.fn(async () => ({ treeMarkdown: '- a', ast: '{"type":"root","children":[]}', flatMarkdown: 'a' })),
    };
    undo = new UndoService();
    const autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    const edits = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
    clipboard = new ClipboardService();
    service = new ItemActionService(state, api as unknown as ApiService, undo, edits, clipboard);
    view = fakeView();
  });

  describe('cycleTask', () => {
    it('saves the next task state and records it for undo', async () => {
      state.setItems([item('a', 1)]);

      await service.cycleTask(state.getItemById('a')!);

      expect(api.cycleTask).toHaveBeenCalledWith('a');
      expect(state.getItemById('a')).toMatchObject({ types: ['task'], properties: { status: 'todo' } });
      expect(undo.popUndo()?.label).toBe('cycle-task');
    });

    it('an item that becomes a task on a daily journal is planned for that day', async () => {
      state.setItems([item('a', 1, { type: 'journal', name: '2026-03-01' })]);

      await service.cycleTask(state.getItemById('a')!);

      expect(api.upsertItemProperty).toHaveBeenCalledWith('a', 'plan', 'date', '2026-03-01');
      expect(state.getItemById('a')?.properties['plan']).toBe('2026-03-01');
    });

    it('an item that already was a task keeps its plan as it is', async () => {
      state.setItems([item('a', 1, { type: 'journal', name: '2026-03-01', types: ['task'] })]);

      await service.cycleTask(state.getItemById('a')!);

      expect(api.upsertItemProperty).not.toHaveBeenCalled();
    });

    it('a transclusion item changes its source item', async () => {
      state.setItems([item('root', 1, { transcludeItemId: 's' })]);
      state.setTranscludedTrees({ s: [item('s', 1)] });

      await service.cycleTask(state.visibleItems()[0]);

      expect(api.cycleTask).toHaveBeenCalledWith('s');
      expect(state.getItemById('s')?.types).toEqual(['task']);
    });
  });

  describe('zoom', () => {
    it('zoomOf reads the stored factor and defaults to 1', () => {
      expect(zoomOf(item('a', 1))).toBe(1);
      expect(zoomOf(item('a', 1, { properties: { zoom: '1.5' } }))).toBe(1.5);
    });

    it('setZoom shows and saves the factor', async () => {
      state.setItems([item('a', 1)]);

      await service.setZoom(state.getItemById('a')!, 1.2);

      expect(state.getItemById('a')?.properties['zoom']).toBe('1.2');
      expect(api.upsertItemProperty).toHaveBeenCalledWith('a', 'zoom', 'number', '1.2');
    });
  });

  describe('toggleCollapsed', () => {
    beforeEach(() => state.setItems([item('a', 1), item('a1', 2), item('b', 1)]));

    it('collapses an item with children, saves it and records it for undo', async () => {
      await service.toggleCollapsed(state.getItemById('a')!);

      expect(state.isCollapsed('a')).toBe(true);
      expect(state.getItemById('a')?.collapsed).toBe(true);
      expect(api.updateItem).toHaveBeenCalledWith('a', { collapsed: true });
      expect(undo.popUndo()?.label).toBe('collapse');
    });

    it('does nothing for an item without children', async () => {
      await service.toggleCollapsed(state.getItemById('b')!);

      expect(api.updateItem).not.toHaveBeenCalled();
    });

    it('puts the screen back when the save fails', async () => {
      api.updateItem.mockRejectedValueOnce(new Error('offline'));

      await service.toggleCollapsed(state.getItemById('a')!);

      expect(state.isCollapsed('a')).toBe(false);
      expect(state.getItemById('a')?.collapsed).toBe(false);
      expect(undo.canUndo()).toBe(false);
    });
  });

  describe('transclusion', () => {
    beforeEach(() => state.setItems([item('a', 1)]));

    it('transclude links the item to the pasted id and reloads the page', async () => {
      await service.transclude(state.getItemById('a')!, '  0123456789abc \n', view);

      expect(api.setTransclusion).toHaveBeenCalledWith('a', '0123456789abc');
      expect(state.getItemById('a')?.transcludeItemId).toBe('0123456789abc');
      expect(view.reloadPage).toHaveBeenCalled();
    });

    it('transclude ignores text that is not an item id', async () => {
      await service.transclude(state.getItemById('a')!, 'some words', view);
      await service.transclude(state.getItemById('a')!, 'short', view);

      expect(api.setTransclusion).not.toHaveBeenCalled();
    });

    it('removeTransclusion unlinks the item and reloads the page', async () => {
      state.setItems([item('a', 1, { transcludeItemId: 's' })]);

      await service.removeTransclusion(state.getItemById('a')!, view);

      expect(api.removeTransclusion).toHaveBeenCalledWith('a');
      expect(state.getItemById('a')?.transcludeItemId).toBeNull();
      expect(view.reloadPage).toHaveBeenCalled();
    });
  });

  describe('archive', () => {
    // * a (1)
    //     * a1 (2)
    // * b (1)
    beforeEach(() => state.setItems([item('a', 1, { body: 'task' }), item('a1', 2), item('b', 1)]));

    it('tags the item, creates #item-archive at the end and moves the subtree under it', async () => {
      await service.archive(state.getItemById('a')!);

      expect(api.updateItem).toHaveBeenCalledWith('a', { body: 'task #archived' });
      expect(api.createItem).toHaveBeenCalledWith('page1', 'after:b', 1, '#item-archive');
      expect(api.moveItems).toHaveBeenCalledWith(['a', 'a1'], 'page1', 'after:new1', 1);
      expect(state.items().map(i => i.id)).toEqual(['b', 'new1', 'a', 'a1']);
    });

    it('reuses an existing #item-archive and does not tag twice', async () => {
      state.setItems([
        item('a', 1, { body: 'done #archived', properties: { tag: ['archived'] } }),
        item('arc', 1, { body: '#item-archive', properties: { tag: ['item-archive'] } }),
      ]);

      await service.archive(state.getItemById('a')!);

      expect(api.updateItem).not.toHaveBeenCalled();
      expect(api.createItem).not.toHaveBeenCalled();
      expect(api.moveItems).toHaveBeenCalledWith(['a'], 'page1', 'after:arc', 1);
    });
  });

  describe('deleteItems', () => {
    it('removes the items from the screen and the server as one undo entry', async () => {
      state.setItems([item('a', 1), item('b', 1), item('c', 1)]);

      await service.deleteItems(['a', 'c']);

      expect(state.items().map(i => i.id)).toEqual(['b']);
      expect(api.deleteItem).toHaveBeenCalledTimes(2);
      expect(undo.popUndo()?.ops.map(op => op.type)).toEqual(['delete', 'delete']);
    });
  });

  describe('copyItems', () => {
    let written: unknown[];

    beforeEach(() => {
      written = [];
      vi.stubGlobal('ClipboardItem', class { constructor(public readonly parts: Record<string, Blob>) {} });
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: { write: vi.fn(async (items: unknown[]) => { written.push(...items); }) },
      });
    });

    afterEach(() => vi.unstubAllGlobals());

    it('writes tree markdown and HTML to the clipboard and keeps the flat markdown', async () => {
      await service.copyItems(['a']);

      expect(api.exportTreeMarkdown).toHaveBeenCalledWith(['a']);
      expect(written).toHaveLength(1);
      expect(Object.keys((written[0] as { parts: Record<string, Blob> }).parts)).toEqual(['text/plain', 'text/html']);
      expect(clipboard.consume('- a')).toBe('a');
    });

    it('does nothing for no items', async () => {
      await service.copyItems([]);

      expect(api.exportTreeMarkdown).not.toHaveBeenCalled();
    });
  });
});
