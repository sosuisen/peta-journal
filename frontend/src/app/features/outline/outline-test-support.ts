import { vi } from 'vitest';
import { Item } from '../../core/models/item.model';
import { item } from '../../core/testing/item-fixture';
import { OutlineView } from './outline-view';

export { item };

/** The hashtags of a body as the server stores them in the tag property. */
function tagsIn(body: string): Record<string, string[]> {
  const tags = [...body.matchAll(/#([\w-]+)/g)].map(m => m[1]);
  return tags.length > 0 ? { tag: tags } : {};
}

/**
 * Records API calls and answers like the server would: an update echoes the fields it got,
 * a create returns a new item (with the client id when one was passed), a move echoes the ids.
 * Both carry the hashtags of the body as a tag property.
 */
export function fakeApi() {
  let created = 0;
  return {
    updateItem: vi.fn(async (id: string, updates: Partial<Item>) =>
      item(id, 1, { ...updates, ast: 'ast:' + (updates.body ?? ''), properties: tagsIn(updates.body ?? '') })),
    createItem: vi.fn(async (pageId: string, _pos: string, level: number, body: string, id?: string) => {
      created++;
      return item(id ?? `new${created}`, level, { pageId, body, idx: `created${created}`, properties: tagsIn(body) });
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
