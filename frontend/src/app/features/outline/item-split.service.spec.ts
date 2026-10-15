import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { ConfigService } from '../../core/services/config.service';
import { HmtidService } from '../../core/services/hmtid.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { ItemEditService } from './item-edit.service';
import { ItemSplitService } from './item-split.service';
import { KeyRelayService } from './key-relay.service';
import { LevelShiftService } from './level-shift.service';
import { fakeApi, fakeView, item, settled } from './outline-test-support';

describe('ItemSplitService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi>;
  let view: ReturnType<typeof fakeView>;
  let service: ItemSplitService;
  let keyRelay: KeyRelayService;

  const bodies = () => state.items().map(i => `${i.id}=${i.body}`);

  beforeEach(() => {
    state = new OutlineStateService();
    api = fakeApi();
    const undo = new UndoService();
    const autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    const edits = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
    const levels = new LevelShiftService(state, api as unknown as ApiService, undo,
      { logicalOutdent: () => false } as unknown as ConfigService);
    const hmtid = { nextId: () => 'client-id' } as HmtidService;
    keyRelay = new KeyRelayService();
    service = new ItemSplitService(state, api as unknown as ApiService, undo, edits, levels, hmtid, keyRelay);
    view = fakeView();
    state.setItems([item('a', 1, { body: 'hello world' }), item('b', 1)]);
    state.addEditingItem('a');
  });

  it('Enter in the middle keeps the text before the cursor and moves the rest to a new item below', async () => {
    await service.handleEnter(state.getItemById('a')!, 5, view);

    expect(bodies()).toEqual(['a=hello', 'new1= world', 'b=body of b']);
    expect(api.createItem).toHaveBeenCalledWith('page1', 'after:a', 1, ' world');
    expect(state.editingItemIds().has('new1')).toBe(true);
    expect(state.editingItemIds().has('a')).toBe(false);
    expect(view.focusEditor).toHaveBeenCalledWith('new1');
    expect(state.consumePendingCursorPosition()).toBe(0);
  });

  it('Enter at the end shows an empty item at once and creates it in the background', async () => {
    await service.handleEnter(state.getItemById('a')!, 'hello world'.length, view);
    await settled();

    expect(state.items().map(i => i.id)).toEqual(['a', 'client-id', 'b']);
    expect(api.createItem).toHaveBeenCalledWith('page1', 'after:a', 1, '', 'client-id');
    expect(state.getItemById('client-id')?.idx).toBe('created1');
    expect(state.focusedItemId()).toBe('client-id');
  });

  it('Enter at the start puts an empty item above and keeps editing the new one', async () => {
    await service.handleEnter(state.getItemById('a')!, 0, view);

    expect(bodies()).toEqual(['new1=', 'a=hello world', 'b=body of b']);
    expect(api.createItem).toHaveBeenCalledWith('page1', 'first', 1, '');
    expect(view.focusEditor).toHaveBeenCalledWith('new1');
  });

  it('a new item after an expanded parent is created as its first child', async () => {
    // * a (1)    ← Enter at end
    //     * c (2)
    state.setItems([item('a', 1, { body: 'parent' }), item('c', 2)]);

    await service.handleEnter(state.getItemById('a')!, 'parent'.length, view);

    expect(api.createItem).toHaveBeenCalledWith('page1', 'after:a', 2, '', 'client-id');
  });

  it('Enter on an empty last child outdents it instead', async () => {
    // * a (1)
    //     * e (2, empty)    ← Enter
    state.setItems([item('a', 1), item('e', 2, { body: '' })]);

    await service.handleEnter(state.getItemById('e')!, 0, view);

    expect(api.createItem).not.toHaveBeenCalled();
    expect(api.shiftItemLevels).toHaveBeenCalledWith(['e'], -1);
    expect(view.focusEditor).toHaveBeenCalledWith('e');
  });

  it('holds the keys pressed until the outline focuses the next editor', async () => {
    view.focusEditor.mockImplementation(async () => { keyRelay.release(); });

    const enter = service.handleEnter(state.getItemById('a')!, 5, view);
    expect(keyRelay.isHolding()).toBe(true);
    await enter;

    expect(view.focusEditor).toHaveBeenCalled();
    expect(keyRelay.isHolding()).toBe(false);
  });

  it('stops holding keys when the split fails', async () => {
    api.createItem.mockRejectedValueOnce(new Error('offline'));

    await expect(service.handleEnter(state.getItemById('a')!, 5, view)).rejects.toThrow('offline');

    expect(keyRelay.isHolding()).toBe(false);
  });

  it('inside a transclusion the new item is created on the source page and the page reloads', async () => {
    // own: root (2, transcludes s); source: s (1, page src)
    const root = item('root', 2, { transcludeItemId: 's', body: 'shown' });
    state.setItems([item('a', 1), root]);
    state.setTranscludedTrees({ s: [item('s', 1, { pageId: 'src', body: 'shown' })] });
    const shownRoot = state.visibleItems().find(i => i.id === 'root')!;

    await service.handleEnter(shownRoot, 'sho'.length, view);

    expect(api.updateItem).toHaveBeenCalledWith('s', { body: 'sho' });
    expect(api.createItem).toHaveBeenCalledWith('src', 'after:s', 1, 'wn');
    expect(view.reloadPage).toHaveBeenCalled();
  });
});
