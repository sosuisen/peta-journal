import { vi } from 'vitest';
import { Item } from '../../core/models/item.model';
import { OutlineView } from './outline-view';

/** A minimal item on page1; `extra` sets the fields a test cares about. */
export function item(id: string, level: number, extra: Partial<Item> = {}): Item {
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

/**
 * Records API calls and answers like the server would: an update echoes the fields it got,
 * a create returns a new item (with the client id when one was passed), a move echoes the ids.
 */
export function fakeApi() {
  let created = 0;
  return {
    updateItem: vi.fn(async (id: string, updates: Partial<Item>) =>
      item(id, 1, { ...updates, ast: 'ast:' + (updates.body ?? '') })),
    createItem: vi.fn(async (pageId: string, _pos: string, level: number, body: string, id?: string) => {
      created++;
      return item(id ?? `new${created}`, level, { pageId, body, idx: `created${created}` });
    }),
    deleteItem: vi.fn(async () => {}),
    shiftItemLevels: vi.fn(async () => []),
    moveItems: vi.fn(async (ids: string[]) => ids.map(id => item(id, 1))),
  };
}

/** Records what the outline component would do; the result satisfies {@link OutlineView}. */
export function fakeView() {
  return {
    pageId: () => 'page1',
    reloadPage: vi.fn(async () => {}),
    reloadPageFromFile: vi.fn((_pageId: string) => {}),
    focusEditor: vi.fn(async (_itemId?: string) => {}),
  };
}

/** Waits for promise callbacks that the code under test does not await. */
export const settled = () => new Promise(resolve => setTimeout(resolve, 0));
