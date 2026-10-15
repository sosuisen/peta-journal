import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Item } from '../../core/models/item.model';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ItemEditService } from './item-edit.service';

function b(id: string, extra: Partial<Item> = {}): Item {
  return {
    id,
    pageId: 'page1',
    dirPath: '',
    name: '',
    type: 'doc',
    createdDate: 0,
    modifiedDate: 0,
    idx: id,
    level: 1,
    body: 'body',
    ast: '',
    collapsed: false,
    types: [],
    properties: {},
    ...extra,
  };
}

describe('ItemEditService', () => {
  let state: OutlineStateService;
  let api: { updateItem: ReturnType<typeof vi.fn> };
  let autoSave: { scheduleItemUpdate: ReturnType<typeof vi.fn>; cancelItem: ReturnType<typeof vi.fn>; flushItem: ReturnType<typeof vi.fn> };
  let service: ItemEditService;
  const item = b('a');

  beforeEach(() => {
    state = new OutlineStateService();
    state.setItems([item]);
    state.addEditingItem('a');
    // The server answers with the body it was given and a rendered AST
    api = { updateItem: vi.fn(async (id: string, u: { body: string }) => b(id, { body: u.body, ast: 'ast:' + u.body, types: ['heading'] })) };
    autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    service = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
  });

  describe('typing', () => {
    it('shows the text at once and schedules the auto-save', () => {
      service.recordTyping(item, 'typed');

      expect(state.getItemById('a')?.body).toBe('typed');
      expect(autoSave.scheduleItemUpdate).toHaveBeenCalledWith('a', 'typed');
      expect(service.isEditInProgress()).toBe(true);
    });

    it('no edit is in progress once every editor is closed, even with a save still pending', () => {
      service.recordTyping(item, 'typed');
      state.removeAllEditingItems();

      expect(service.isEditInProgress()).toBe(false);
    });

    it('a transclusion root schedules the save for its source item', () => {
      const root = b('r', { transcludeItemId: 'src' });
      state.setItems([root]);

      service.recordTyping(root, 'typed');

      expect(autoSave.scheduleItemUpdate).toHaveBeenCalledWith('src', 'typed');
    });
  });

  describe('finishEdit', () => {
    it('saves typed text once, cancelling the auto-save, and closes the editor', async () => {
      service.recordTyping(item, 'typed');

      await service.finishEdit(item);

      expect(autoSave.cancelItem).toHaveBeenCalledWith('a');
      expect(api.updateItem).toHaveBeenCalledTimes(1);
      expect(api.updateItem).toHaveBeenCalledWith('a', { body: 'typed' });
      expect(state.getItemById('a')?.ast).toBe('ast:typed');
      expect(state.editingItemIds().has('a')).toBe(false);
      expect(service.isEditInProgress()).toBe(false);
    });

    it('closes the editor of a clean item without a save', async () => {
      await service.finishEdit(item);

      expect(api.updateItem).not.toHaveBeenCalled();
      expect(state.editingItemIds().has('a')).toBe(false);
    });

    it('closes an open code fence even when nothing was typed since', async () => {
      state.updateItem('a', { body: '```js\ncode' });

      await service.finishEdit(item);

      expect(api.updateItem).toHaveBeenCalledWith('a', { body: '```js\ncode\n```' });
    });

    it('is ignored while a handover is saving the item itself', async () => {
      service.recordTyping(item, 'typed');

      await service.handOverEdit(async () => {
        await service.finishEdit(item);
        expect(api.updateItem).not.toHaveBeenCalled();
        expect(state.editingItemIds().has('a')).toBe(true);
      });
    });

    it('is ignored when the edit was already ended by another handler', async () => {
      state.removeEditingItem('a');
      service.recordTyping(item, 'typed');

      await service.finishEdit(item);

      expect(api.updateItem).not.toHaveBeenCalled();
    });
  });

  describe('handOverEdit', () => {
    it('lets blur end edits again after the handler failed', async () => {
      await expect(service.handOverEdit(async () => { throw new Error('boom'); })).rejects.toThrow('boom');

      await service.finishEdit(item);
      expect(state.editingItemIds().has('a')).toBe(false);
    });
  });

  describe('saving a transclusion root', () => {
    it('writes to the source and copies the rendering back to the shown item', async () => {
      const root = b('r', { transcludeItemId: 'src', body: 'shown' });
      state.setItems([root]);
      state.addEditingItem('r');
      service.recordTyping(root, 'typed');

      await service.saveItemIfDirty(root);

      expect(api.updateItem).toHaveBeenCalledWith('src', { body: 'typed' });
      expect(state.getItemById('r')?.ast).toBe('ast:typed');
      expect(state.getItemById('r')?.types).toEqual(['heading']);
      expect(state.editingItemIds().has('r')).toBe(false);
    });
  });

  describe('pending text before another operation', () => {
    it('flushPendingEdit sends it through the auto-save right away', async () => {
      service.recordTyping(item, 'typed');

      await service.flushPendingEdit(item);

      expect(autoSave.flushItem).toHaveBeenCalledWith('a');
      expect(service.isEditInProgress()).toBe(true); // the editor is still open
      await service.flushPendingEdit(item);
      expect(autoSave.flushItem).toHaveBeenCalledTimes(1);
    });

    it('discardPendingEditOfFocusedItem cancels the save of the focused item\'s target', () => {
      const root = b('r', { transcludeItemId: 'src' });
      state.setItems([root]);
      state.setFocusedItem('r');
      service.recordTyping(root, 'typed');

      service.discardPendingEditOfFocusedItem();

      expect(autoSave.cancelItem).toHaveBeenCalledWith('src');
    });
  });
});
