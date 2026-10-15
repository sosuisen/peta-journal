import { beforeEach, describe, expect, it } from 'vitest';
import { item } from '../testing/item-fixture';
import { OutlineStateService } from './outline-state.service';

describe('OutlineStateService', () => {
  let state: OutlineStateService;

  const ids = () => state.items().map(i => i.id);
  const visible = () => state.visibleItems().map(i => `${i.id}:${i.level}`);

  beforeEach(() => {
    state = new OutlineStateService();
  });

  describe('inserting and removing', () => {
    beforeEach(() => state.setItems([item('a', 1), item('b', 1)]));

    it('addItem puts an item first, last or after another one', () => {
      state.addItem(item('x', 1), 'first');
      state.addItem(item('y', 1), 'last');
      state.addItem(item('z', 1), 'after:a');

      expect(ids()).toEqual(['x', 'a', 'z', 'b', 'y']);
    });

    it('an unknown anchor puts the item at the end', () => {
      state.addItem(item('x', 1), 'after:gone');

      expect(ids()).toEqual(['a', 'b', 'x']);
    });

    it('addItems keeps the order of the inserted items', () => {
      state.addItems([item('x', 1), item('y', 2)], 'after:a');

      expect(ids()).toEqual(['a', 'x', 'y', 'b']);
    });

    it('replaceItems moves items in one step, so the anchor is searched without them', () => {
      state.setItems([item('a', 1), item('b', 1), item('c', 1)]);

      state.replaceItems(['a'], [item('a', 1, { body: 'moved' })], 'after:c');

      expect(ids()).toEqual(['b', 'c', 'a']);
      expect(state.getItemById('a')?.body).toBe('moved');
    });

    it('deleteItems removes several items at once', () => {
      state.deleteItems(['a', 'b']);

      expect(ids()).toEqual([]);
    });
  });

  describe('updating', () => {
    it('updateItem merges the given fields into an own item', () => {
      state.setItems([item('a', 1)]);

      state.updateItem('a', { body: 'new', level: 2 });

      expect(state.getItemById('a')).toMatchObject({ body: 'new', level: 2, idx: 'a' });
    });

    it('updateItem and deleteItem reach into transcluded trees too', () => {
      state.setItems([item('root', 1, { transcludeItemId: 's' })]);
      state.setTranscludedTrees({ s: [item('s', 1), item('s1', 2)] });

      state.updateItem('s1', { body: 'changed' });
      expect(state.getItemById('s1')?.body).toBe('changed');

      state.deleteItem('s1');
      expect(state.getItemById('s1')).toBeUndefined();
      expect(state.transcludedTrees().get('s')?.map(i => i.id)).toEqual(['s']);
    });
  });

  describe('tree questions', () => {
    // * a (1)
    //     * a1 (2)
    //         * a1x (3)
    //     * a2 (2)
    // * b (1)
    beforeEach(() => state.setItems([item('a', 1), item('a1', 2), item('a1x', 3), item('a2', 2), item('b', 1)]));

    it('getChildren returns every descendant and stops at the next sibling', () => {
      expect(state.getChildren('a').map(i => i.id)).toEqual(['a1', 'a1x', 'a2']);
      expect(state.getChildren('a1').map(i => i.id)).toEqual(['a1x']);
      expect(state.getChildren('b')).toEqual([]);
    });

    it('getItemWithChildren starts with the item itself; an unknown id gives nothing', () => {
      expect(state.getItemWithChildren('a1').map(i => i.id)).toEqual(['a1', 'a1x']);
      expect(state.getItemWithChildren('gone')).toEqual([]);
    });

    it('hasChildren looks at the next item only', () => {
      expect(state.hasChildren('a')).toBe(true);
      expect(state.hasChildren('a2')).toBe(false);
      expect(state.hasChildren('b')).toBe(false);
    });

    it('ownItemIds and isOwnItem cover the loaded pages but not transcluded descendants', () => {
      state.setTranscludedTrees({ s: [item('s', 1), item('s1', 2)] });

      expect(state.isOwnItem('a1')).toBe(true);
      expect(state.isOwnItem('s1')).toBe(false);
      expect(state.ownItemIds().size).toBe(5);
    });
  });

  describe('collapsing', () => {
    // * a (1, collapsed)
    //     * a1 (2)
    // * b (1)
    beforeEach(() => state.setItems([item('a', 1, { collapsed: true }), item('a1', 2), item('b', 1)]));

    it('setItems reads the collapsed flags and visibleItems hides the children', () => {
      expect(state.isCollapsed('a')).toBe(true);
      expect(visible()).toEqual(['a:1', 'b:1']);
    });

    it('toggleCollapsed shows the children again', () => {
      state.toggleCollapsed('a');

      expect(state.isCollapsed('a')).toBe(false);
      expect(visible()).toEqual(['a:1', 'a1:2', 'b:1']);
    });
  });

  describe('transclusions', () => {
    // own:    root (2, transcludes s), b (1)
    // source: s (1) > s1 (2) > s1x (3), s2 (2)
    beforeEach(() => {
      state.setItems([item('root', 2, { transcludeItemId: 's', body: 'placeholder' }), item('b', 1)]);
      state.setTranscludedTrees({
        s: [item('s', 1, { body: 'source', types: ['heading'] }), item('s1', 2), item('s1x', 3), item('s2', 2)],
      });
    });

    it('shows the source root in place of the transclusion item and its descendants below, re-levelled', () => {
      expect(visible()).toEqual(['root:2', 's1:3', 's1x:4', 's2:3', 'b:1']);
      expect(state.visibleItems()[0]).toMatchObject({ id: 'root', body: 'source', types: ['heading'] });
    });

    it('a collapsed transclusion item hides the descendants', () => {
      state.toggleCollapsed('root');

      expect(visible()).toEqual(['root:2', 'b:1']);
    });

    it('a collapsed transcluded descendant hides its own subtree only', () => {
      state.toggleCollapsed('s1');

      expect(visible()).toEqual(['root:2', 's1:3', 's2:3', 'b:1']);
      expect(state.visibleItems()[1].collapsed).toBe(true);
    });

    it('a transclusion whose tree is not loaded is shown as it is', () => {
      state.setTranscludedTrees({});

      expect(visible()).toEqual(['root:2', 'b:1']);
      expect(state.visibleItems()[0].body).toBe('placeholder');
    });

    it('hasChildren is true for a transclusion root with descendants and for a transcluded parent', () => {
      expect(state.hasChildren('root')).toBe(true);
      expect(state.hasChildren('s1')).toBe(true);
      expect(state.hasChildren('s2')).toBe(false);
    });

    it('getItemById finds transcluded items', () => {
      expect(state.getItemById('s1x')?.level).toBe(3);
    });

    it('setTranscludedTrees takes the collapsed flags of the trees', () => {
      state.setTranscludedTrees({ s: [item('s', 1), item('s1', 2, { collapsed: true }), item('s1x', 3)] });

      expect(visible()).toEqual(['root:2', 's1:3', 'b:1']);
    });

    it('mergeTranscludedTrees adds trees and replaces a known one', () => {
      state.mergeTranscludedTrees({ t: [item('t', 1)], s: [item('s', 1, { body: 'replaced' })] });

      expect([...state.transcludedTrees().keys()]).toEqual(['s', 't']);
      expect(state.visibleItems()[0].body).toBe('replaced');
    });
  });

  describe('headings', () => {
    it('lists heading items with the level from the AST and the text without the marks', () => {
      state.setItems([
        item('h', 1, { types: ['heading'], body: '## Title', ast: JSON.stringify({ children: [{ type: 'heading', level: 2 }] }) }),
        item('p', 1, { body: 'text' }),
        item('broken', 1, { types: ['heading'], body: '# Loose', ast: 'not json' }),
      ]);

      expect(state.headings()).toEqual([
        { id: 'h', text: 'Title', level: 2 },
        { id: 'broken', text: 'Loose', level: 1 },
      ]);
    });
  });

  describe('editing and focus', () => {
    it('keeps the set of items whose editor is open', () => {
      state.addEditingItem('a');
      state.addEditingItem('b');
      state.removeEditingItem('a');
      expect([...state.editingItemIds()]).toEqual(['b']);

      state.removeAllEditingItems();
      expect(state.editingItemIds().size).toBe(0);
    });

    it('the pending cursor position is read once', () => {
      state.setFocusedItem('a', 7);

      expect(state.focusedItemId()).toBe('a');
      expect(state.consumePendingCursorPosition()).toBe(7);
      expect(state.consumePendingCursorPosition()).toBeNull();
    });

    it('a pending mouse drag is read once', () => {
      state.setPendingMouseDrag(true);

      expect(state.consumePendingMouseDrag()).toBe(true);
      expect(state.consumePendingMouseDrag()).toBe(false);
    });

    it('scroll and edit requests stay until they are cleared', () => {
      state.requestScrollToItem('a');
      state.requestEditItem('b');
      expect(state.scrollRequestItemId()).toBe('a');
      expect(state.pendingEditItemId()).toBe('b');

      state.clearScrollRequest();
      state.clearPendingEditItem();
      expect(state.scrollRequestItemId()).toBeNull();
      expect(state.pendingEditItemId()).toBeNull();
    });
  });

  describe('selection', () => {
    it('clearSelection empties the set and leaves selection mode', () => {
      state.setSelectedItemIds(new Set(['a']));
      state.setItemSelectionMode(true);

      state.clearSelection();

      expect(state.selectedItemIds().size).toBe(0);
      expect(state.itemSelectionMode()).toBe(false);
    });
  });
});
