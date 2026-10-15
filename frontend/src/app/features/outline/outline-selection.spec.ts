import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { OutlineSelection, SelectionCommands } from './outline-selection';
import { item, settled } from './outline-test-support';

describe('OutlineSelection', () => {
  let state: OutlineStateService;
  let commands: { [K in keyof SelectionCommands]: ReturnType<typeof vi.fn<SelectionCommands[K]>> };
  let selection: OutlineSelection;

  const selected = () => [...state.selectedItemIds()].sort();
  const press = (key: string, modifiers: KeyboardEventInit = {}) =>
    document.dispatchEvent(new KeyboardEvent('keydown', { key, ...modifiers }));

  // * a (1)
  //     * a1 (2)
  //     * a2 (2)
  // * b (1)
  // * c (1)
  beforeEach(() => {
    state = new OutlineStateService();
    state.setItems([item('a', 1), item('a1', 2), item('a2', 2), item('b', 1), item('c', 1)]);
    commands = {
      undo: vi.fn<() => void>(), redo: vi.fn<() => void>(),
      deleteSelected: vi.fn<() => void>(), moveSelected: vi.fn<(direction: 'up' | 'down') => void>(),
      indentSelected: vi.fn<() => void>(), outdentSelected: vi.fn<() => void>(),
      copySelected: vi.fn<() => void>(), cutSelected: vi.fn<() => void>(),
    };
    selection = new OutlineSelection(state, commands);
  });

  afterEach(() => selection.dispose());

  describe('starting', () => {
    it('from an editor selects only that item and leaves edit mode', () => {
      state.addEditingItem('a');

      selection.startFromEditor(state.getItemById('a')!);

      expect(selected()).toEqual(['a']);
      expect(state.itemSelectionMode()).toBe(true);
      expect(state.editingItemIds().size).toBe(0);
    });

    it('selectSubtree takes the item with its children', () => {
      selection.selectSubtree('a');

      expect(selected()).toEqual(['a', 'a1', 'a2']);
    });
  });

  describe('Shift+Arrow', () => {
    it('Shift+Down adds the item below with its subtree', () => {
      selection.startFromEditor(state.getItemById('b')!);

      press('ArrowDown', { shiftKey: true });

      expect(selected()).toEqual(['b', 'c']);
    });

    it('Shift+Up from a level-1 item pulls in the whole subtree above', () => {
      selection.startFromEditor(state.getItemById('b')!);

      press('ArrowUp', { shiftKey: true });

      expect(selected()).toEqual(['a', 'a1', 'a2', 'b']);
    });

    it('Shift+Down after Shift+Up drops the whole subtree that was pulled in', () => {
      selection.startFromEditor(state.getItemById('b')!);
      press('ArrowUp', { shiftKey: true });

      press('ArrowDown', { shiftKey: true });

      expect(selected()).toEqual(['b']);
    });

    it('Shift+Down undoes one Shift+Up at a time', () => {
      selection.startFromEditor(state.getItemById('c')!);
      press('ArrowUp', { shiftKey: true });
      press('ArrowUp', { shiftKey: true });
      expect(selected()).toEqual(['a', 'a1', 'a2', 'b', 'c']);

      press('ArrowDown', { shiftKey: true });
      expect(selected()).toEqual(['b', 'c']);

      press('ArrowDown', { shiftKey: true });
      expect(selected()).toEqual(['c']);
    });

    it('Shift+Down from a child anchor shrinks back to the anchor', () => {
      selection.startFromEditor(state.getItemById('a1')!);
      press('ArrowUp', { shiftKey: true });
      expect(selected()).toEqual(['a', 'a1', 'a2']);

      press('ArrowDown', { shiftKey: true });

      expect(selected()).toEqual(['a1']);
    });

    it('stops at the ends of the page', () => {
      selection.startFromEditor(state.getItemById('c')!);

      press('ArrowDown', { shiftKey: true });

      expect(selected()).toEqual(['c']);
    });
  });

  describe('mouse drag', () => {
    it('selects from the anchor to the item the mouse enters', () => {
      selection.startDrag(state.getItemById('a')!);
      expect(selected()).toEqual(['a', 'a1', 'a2']);

      selection.extendDragTo(state.getItemById('c')!);

      expect(selected()).toEqual(['a', 'a1', 'a2', 'b', 'c']);
    });

    it('ignores mouse movement when no drag is in progress', () => {
      selection.startFromEditor(state.getItemById('a')!);

      selection.extendDragTo(state.getItemById('c')!);

      expect(selected()).toEqual(['a']);
    });

    it('a click right after the mouse is released keeps the selection; a later click ends it', async () => {
      selection.startDrag(state.getItemById('b')!);

      document.dispatchEvent(new MouseEvent('mouseup'));
      document.dispatchEvent(new MouseEvent('click'));
      expect(selected()).toEqual(['b']);

      await settled();
      document.dispatchEvent(new MouseEvent('click'));
      expect(state.itemSelectionMode()).toBe(false);
    });
  });

  describe('keys in selection mode', () => {
    beforeEach(() => selection.startFromEditor(state.getItemById('b')!));

    it('Escape ends the selection and stops listening', () => {
      press('Escape');

      expect(state.itemSelectionMode()).toBe(false);
      press('Delete');
      expect(commands.deleteSelected).not.toHaveBeenCalled();
    });

    it.each([
      ['Delete', {}, 'deleteSelected', []],
      ['Backspace', {}, 'deleteSelected', []],
      ['z', { ctrlKey: true }, 'undo', []],
      ['Z', { ctrlKey: true, shiftKey: true }, 'redo', []],
      ['ArrowUp', { shiftKey: true, altKey: true }, 'moveSelected', ['up']],
      ['ArrowDown', { shiftKey: true, altKey: true }, 'moveSelected', ['down']],
      ['ArrowRight', { shiftKey: true, altKey: true }, 'indentSelected', []],
      ['ArrowLeft', { shiftKey: true, altKey: true }, 'outdentSelected', []],
      ['Tab', {}, 'indentSelected', []],
      ['Tab', { shiftKey: true }, 'outdentSelected', []],
      ['c', { ctrlKey: true }, 'copySelected', []],
      ['x', { ctrlKey: true }, 'cutSelected', []],
    ] as const)('%s %o runs %s', (key, modifiers, command, args) => {
      press(key, modifiers);

      expect(commands[command]).toHaveBeenCalledWith(...args);
    });

    it('the copy and cut events of the document are taken over', () => {
      document.dispatchEvent(new Event('copy'));
      document.dispatchEvent(new Event('cut'));

      expect(commands.copySelected).toHaveBeenCalled();
      expect(commands.cutSelected).toHaveBeenCalled();
    });
  });
});
