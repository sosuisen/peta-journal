import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ItemEditService } from './item-edit.service';
import { FileReloadService } from './file-reload.service';
import { item, settled } from './outline-test-support';

describe('FileReloadService', () => {
  let state: OutlineStateService;
  let getPageById: ReturnType<typeof vi.fn>;
  let editing: boolean;
  let service: FileReloadService;

  const ids = () => state.items().map(i => i.id);

  // page1: a, b    page2: c
  beforeEach(() => {
    state = new OutlineStateService();
    state.setItems([item('a', 1), item('b', 1), item('c', 1, { pageId: 'page2' })]);
    getPageById = vi.fn(async (pageId: string) => ({
      items: [item('a', 1, { pageId, body: 'from file' }), item('n', 1, { pageId })],
      transcludedTrees: { s: [item('s', 1)] },
    }));
    editing = false;
    const edits = { isEditInProgress: () => editing } as unknown as ItemEditService;
    service = new FileReloadService(state, { getPageById } as unknown as ApiService, edits);
  });

  it('replaces the items of the page in place and keeps the other pages', async () => {
    await service.reloadPage('page1');

    expect(ids()).toEqual(['a', 'n', 'c']);
    expect(state.getItemById('a')?.body).toBe('from file');
    expect(state.transcludedTrees().has('s')).toBe(true);
  });

  it('ignores a page the outline does not show', async () => {
    await service.reloadPage('elsewhere');

    expect(getPageById).not.toHaveBeenCalled();
  });

  it('waits while the user is editing and reloads when asked to flush', async () => {
    editing = true;
    await service.reloadPage('page1');
    expect(getPageById).not.toHaveBeenCalled();
    expect(service.hasPending()).toBe(true);

    service.flushPending();
    expect(getPageById).not.toHaveBeenCalled();

    editing = false;
    service.flushPending();
    await settled();
    expect(getPageById).toHaveBeenCalledWith('page1');
    expect(service.hasPending()).toBe(false);
  });

  it('keeps the items when the server fails', async () => {
    getPageById.mockRejectedValueOnce(new Error('offline'));
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    await service.reloadPage('page1');

    expect(ids()).toEqual(['a', 'b', 'c']);
    error.mockRestore();
  });
});
