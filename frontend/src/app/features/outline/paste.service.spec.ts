import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { ClipboardService } from '../../core/services/clipboard.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { ItemEditService } from './item-edit.service';
import { PasteService } from './paste.service';
import { fakeApi, fakeView, item } from './outline-test-support';

describe('PasteService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi> & {
    parseFlatMarkdown: ReturnType<typeof vi.fn>;
    importTreeMarkdown: ReturnType<typeof vi.fn>;
    uploadAsset: ReturnType<typeof vi.fn>;
  };
  let undo: UndoService;
  let clipboard: ClipboardService;
  let view: ReturnType<typeof fakeView>;
  let service: PasteService;

  const items = () => state.items().map(i => `${i.id}:${i.level}=${i.body}`);
  /** Splits like the server: one item per line, "  " in front means one level deeper. */
  const splitLines = async (text: string) => ({
    items: text === '' ? [] : text.split('\n').map(line => ({ body: line.trimStart(), level: 1 + (line.length - line.trimStart().length) / 2 })),
  });

  beforeEach(() => {
    state = new OutlineStateService();
    api = {
      ...fakeApi(),
      parseFlatMarkdown: vi.fn(splitLines),
      importTreeMarkdown: vi.fn(async () => ({ flatMarkdown: 'from tree' })),
      uploadAsset: vi.fn(async () => ({ fileName: 'pic.png' })),
    };
    undo = new UndoService();
    const autoSave = { scheduleItemUpdate: vi.fn(), cancelItem: vi.fn(), flushItem: vi.fn(async () => null) };
    const edits = new ItemEditService(state, api as unknown as ApiService, autoSave as unknown as AutoSaveService);
    clipboard = new ClipboardService();
    service = new PasteService(state, api as unknown as ApiService, undo, edits, clipboard);
    view = fakeView();
    state.setItems([item('a', 1, { body: 'ab|cd' }), item('z', 1)]);
  });

  describe('text', () => {
    it('one piece goes into the item at the cursor', async () => {
      await service.pasteText(state.getItemById('a')!, { text: 'X', html: '', cursorPos: 2 }, view);

      expect(items()).toEqual(['a:1=abX|cd', 'z:1=body of z']);
      expect(api.createItem).not.toHaveBeenCalled();
      expect(view.focusEditor).toHaveBeenCalledWith();
    });

    it('several pieces become items below, the last one taking the text after the cursor', async () => {
      await service.pasteText(state.getItemById('a')!, { text: 'one\n  two\nthree', html: '', cursorPos: 2 }, view);

      expect(items()).toEqual(['a:1=abone', 'new1:2=two', 'new2:1=three|cd', 'z:1=body of z']);
      expect(api.createItem).toHaveBeenNthCalledWith(1, 'page1', 'after:a', 2, 'two');
      expect(api.createItem).toHaveBeenNthCalledWith(2, 'page1', 'after:new1', 1, 'three|cd');
    });

    it('records one undo entry with the update and the created items', async () => {
      await service.pasteText(state.getItemById('a')!, { text: 'one\ntwo', html: '', cursorPos: 2 }, view);

      const entry = undo.popUndo();
      expect(entry?.label).toBe('paste');
      expect(entry?.ops.map(op => op.type)).toEqual(['update', 'create']);
      expect(entry?.cursorBefore).toEqual({ itemId: 'a', position: 2 });
    });

    it('text copied from this app is pasted from the stored flat markdown', async () => {
      clipboard.store('flat', '- tree');

      await service.pasteText(state.getItemById('a')!, { text: '- tree', html: '', cursorPos: 5 }, view);

      expect(api.parseFlatMarkdown).toHaveBeenCalledWith('flat');
      expect(api.importTreeMarkdown).not.toHaveBeenCalled();
    });

    it('tree markdown from elsewhere is converted by the server first', async () => {
      await service.pasteText(state.getItemById('a')!, { text: '- x\n  - y', html: '', cursorPos: 5 }, view);

      expect(api.importTreeMarkdown).toHaveBeenCalledWith('- x\n  - y');
      expect(api.parseFlatMarkdown).toHaveBeenCalledWith('from tree');
    });

    it('falls back to the raw text when the conversion fails', async () => {
      api.importTreeMarkdown.mockRejectedValueOnce(new Error('bad'));

      await service.pasteText(state.getItemById('a')!, { text: '- x', html: '', cursorPos: 5 }, view);

      expect(api.parseFlatMarkdown).toHaveBeenCalledWith('- x');
    });

    it('plain text inside an open code fence is not split', async () => {
      state.setItems([item('code', 1, { body: '```\nlet' })]);

      await service.pasteText(state.getItemById('code')!, { text: 'a\nb', html: '', cursorPos: 7 }, view);

      expect(api.parseFlatMarkdown).not.toHaveBeenCalled();
      // The save closes the fence that is still open
      expect(state.getItemById('code')?.body).toBe('```\nleta\nb\n```');
    });

    it('nothing happens when the text has no items', async () => {
      await service.pasteText(state.getItemById('a')!, { text: '', html: '', cursorPos: 2 }, view);

      expect(api.updateItem).not.toHaveBeenCalled();
      expect(undo.canUndo()).toBe(false);
    });

    it('a failed creation drops the undo entry', async () => {
      api.createItem.mockRejectedValueOnce(new Error('offline'));

      await service.pasteText(state.getItemById('a')!, { text: 'one\ntwo', html: '', cursorPos: 2 }, view);

      expect(undo.canUndo()).toBe(false);
    });
  });

  describe('image', () => {
    it('uploads the file, links it at the cursor and puts the cursor after the link', async () => {
      const file = new File(['x'], 'pic.png');

      await service.pasteImage(state.getItemById('a')!, { file, cursorPos: 2 }, view);

      expect(api.uploadAsset).toHaveBeenCalledWith(file);
      expect(state.getItemById('a')?.body).toBe('ab![pic.png](../assets/pic.png)|cd');
      expect(state.consumePendingCursorPosition()).toBe(2 + '![pic.png](../assets/pic.png)'.length);
      expect(view.focusEditor).toHaveBeenCalledWith('a');
    });
  });
});
