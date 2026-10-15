import { Item } from '../models/item.model';

/** A minimal item on page1 for tests; `extra` sets the fields a test cares about. */
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
