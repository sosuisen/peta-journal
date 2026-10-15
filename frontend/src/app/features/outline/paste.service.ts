import { Injectable } from '@angular/core';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { ClipboardService } from '../../core/services/clipboard.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemEditService } from './item-edit.service';
import { hasUnclosedCodeFence } from './item-editor';
import { OutlineView } from './outline-view';

/** What an editor reports about a text paste. */
export interface TextPaste {
  text: string;
  html: string;
  cursorPos: number;
}

/** What an editor reports about an image paste. */
export interface ImagePaste {
  file: File;
  cursorPos: number;
}

/** A piece of pasted text that becomes an item; the level is relative to the item pasted into. */
interface PastedItem {
  body: string;
  level: number;
}

/**
 * Pastes into an item editor. Text is split into items at the same block boundaries as a
 * markdown file, the first piece joins the text before the cursor and the last piece the
 * text after it; the pieces in between become new items below. An image is uploaded and
 * linked at the cursor.
 */
@Injectable({ providedIn: 'root' })
export class PasteService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly edits: ItemEditService,
    private readonly clipboard: ClipboardService,
  ) {}

  /**
   * Pastes text into an item. Supports plain text, tree markdown, and flat markdown that
   * was copied from this app.
   * @param item - The item where the paste occurred
   * @param paste - The pasted text, HTML, and cursor position at the time of paste
   * @param view - The outline, which focuses an editor when the paste is done
   */
  async pasteText(item: Item, paste: TextPaste, view: OutlineView): Promise<void> {
    let textToParse = paste.text;
    let isPlainText = true;

    // Check 1: If we have stored flat markdown for this clipboard content, use it
    const flatMd = this.clipboard.consume(paste.text);
    if (flatMd !== null) {
      textToParse = flatMd;
      isPlainText = false;
    }
    // Check 2: If text looks like tree markdown (starts with "- "), convert it
    else if (paste.text.trimStart().startsWith('- ')) {
      try {
        const result = await this.api.importTreeMarkdown(paste.text);
        textToParse = result.flatMarkdown;
        isPlainText = false;
      } catch (e) {
        console.error('Failed to import tree markdown, using raw text:', e);
      }
    }

    // Plain text pasted inside an open code fence stays in this item as it is
    const skipSplit = isPlainText && hasUnclosedCodeFence(item.body.substring(0, paste.cursorPos));
    const parsedItems = skipSplit
      ? [{ body: textToParse, level: 0 }]
      : await this.splitTextIntoItems(textToParse);

    if (parsedItems.length === 0) return;

    const originalItem = { ...item };

    // Split existing text at caret position
    const before = item.body.substring(0, paste.cursorPos);
    const after = item.body.substring(paste.cursorPos);

    // Merge existing text around pasted content
    parsedItems[0].body = before + parsedItems[0].body;
    parsedItems[parsedItems.length - 1].body = parsedItems[parsedItems.length - 1].body + after;

    try {
      this.undo.begin('paste', { itemId: item.id, position: paste.cursorPos });

      // First item replaces current editing item
      const firstItem = parsedItems[0];
      await this.edits.saveBody(item.id, firstItem.body);
      this.edits.markDirty(item.id);

      const afterFirstItem = this.state.getItemById(item.id);
      if (afterFirstItem) {
        this.undo.pushUpdate(originalItem, { ...afterFirstItem });
      }

      // Add remaining items after the current item
      let prevItemId = item.id;
      for (let i = 1; i < parsedItems.length; i++) {
        const parsedItem = parsedItems[i];
        const afterPos: ItemPosition = `after:${prevItemId}`;
        const newItem = await this.api.createItem(
          item.pageId,
          afterPos,
          item.level + parsedItem.level,
          parsedItem.body
        );
        this.state.addItems([newItem], afterPos);

        const finalNewItem = this.state.getItemById(newItem.id);
        this.undo.pushCreate(finalNewItem ?? newItem);

        prevItemId = newItem.id;
      }

      this.undo.commit();

      // Focus the last created item or the current item
      view.focusEditor();
    } catch (e) {
      console.error('Failed to process paste:', e);
      this.undo.abort();
    }
  }

  /**
   * Uploads a pasted image and puts a link to it at the cursor.
   * @param item - The item where the paste occurred
   * @param paste - The image file and the cursor position
   * @param view - The outline, which puts the cursor after the link
   */
  async pasteImage(item: Item, paste: ImagePaste, view: OutlineView): Promise<void> {
    try {
      const result = await this.api.uploadAsset(paste.file);
      const fileName = result.fileName;
      const markdown = `![${fileName}](../assets/${fileName})`;

      const before = item.body.substring(0, paste.cursorPos);
      const after = item.body.substring(paste.cursorPos);
      const newBody = before + markdown + after;

      await this.edits.saveBody(item.id, newBody);
      this.edits.markDirty(item.id);

      // Place cursor after the inserted markdown
      const newPos = paste.cursorPos + markdown.length;
      this.state.setFocusedItem(item.id, newPos);
      view.focusEditor(item.id);
    } catch (e) {
      console.error('Failed to paste image:', e);
    }
  }

  /**
   * Splits text into items with the server-side Flat Markdown parser, so that pasted text
   * is cut at the same block boundaries as a markdown file. Levels are made relative to the
   * first item (0-based), because the caller places them under the current item.
   */
  private async splitTextIntoItems(text: string): Promise<PastedItem[]> {
    const parsed = (await this.api.parseFlatMarkdown(text)).items;
    if (parsed.length === 0) return [];
    const topLevel = parsed[0].level;
    return parsed.map(p => ({ body: p.body, level: Math.max(0, p.level - topLevel) }));
  }
}
