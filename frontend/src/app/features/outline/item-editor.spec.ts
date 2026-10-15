import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ItemEditorComponent } from './item-editor';
import { DeferredKey, KeyRelayService } from './key-relay.service';

describe('ItemEditorComponent', () => {
  let editor: ItemEditorComponent;
  let textarea: HTMLTextAreaElement;
  let keyRelay: KeyRelayService;
  let bodies: string[];
  let indents: number;

  const key = (k: string, init: Partial<DeferredKey> = {}): DeferredKey =>
    ({ key: k, code: '', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, ...init });

  beforeEach(() => {
    TestBed.configureTestingModule({});
    const fixture = TestBed.createComponent(ItemEditorComponent);
    fixture.componentRef.setInput('body', 'ab');
    fixture.componentRef.setInput('itemId', 'a');
    fixture.componentRef.setInput('types', []);
    fixture.componentRef.setInput('properties', {});
    fixture.detectChanges();
    editor = fixture.componentInstance;
    textarea = fixture.nativeElement.querySelector('textarea');
    keyRelay = TestBed.inject(KeyRelayService);
    bodies = [];
    indents = 0;
    editor.bodyChange.subscribe(b => bodies.push(b));
    editor.indentRequested.subscribe(() => indents++);
  });

  describe('keys while the relay holds', () => {
    it('are handed to the relay instead of being handled', () => {
      keyRelay.holdUntilFocus();
      const tab = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });

      textarea.dispatchEvent(tab);

      expect(tab.defaultPrevented).toBe(true);
      expect(indents).toBe(0);
      expect(keyRelay.release().map(k => k.key)).toEqual(['Tab']);
    });

    it('shortcuts with Ctrl are left to the browser, because they cannot be replayed', () => {
      keyRelay.holdUntilFocus();
      const paste = new KeyboardEvent('keydown', { key: 'v', ctrlKey: true, bubbles: true, cancelable: true });

      textarea.dispatchEvent(paste);

      expect(paste.defaultPrevented).toBe(false);
      expect(keyRelay.release()).toEqual([]);
    });

    it('are handled as usual once the relay released them', () => {
      keyRelay.holdUntilFocus();
      keyRelay.release();

      textarea.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }));

      expect(indents).toBe(1);
    });
  });

  describe('replayKeys', () => {
    it('types characters at the caret and reports the new body', () => {
      textarea.setSelectionRange(1, 1);

      editor.replayKeys([key('X'), key('Y')]);

      expect(textarea.value).toBe('aXYb');
      expect(bodies).toEqual(['aXb', 'aXYb']);
    });

    it('runs other keys through the keydown handler', () => {
      editor.replayKeys([key('Tab')]);

      expect(indents).toBe(1);
    });

    it('does what the browser would do for Backspace, Delete and the arrows', () => {
      textarea.setSelectionRange(2, 2);

      editor.replayKeys([key('Backspace'), key('X'), key('ArrowLeft'), key('ArrowLeft'), key('Delete'), key('End'), key('Y')]);

      expect(textarea.value).toBe('XY');
    });

    it('Backspace at the start still goes to the keydown handler, which merges items', () => {
      let merges = 0;
      editor.backspaceAtStart.subscribe(() => merges++);
      textarea.setSelectionRange(0, 0);

      editor.replayKeys([key('Backspace')]);

      expect(merges).toBe(1);
      expect(textarea.value).toBe('ab');
    });

    it('keys after a replayed Enter are held for the item that Enter creates', () => {
      editor.enterPressed.subscribe(() => keyRelay.holdUntilFocus());
      textarea.setSelectionRange(2, 2);

      editor.replayKeys([key('Enter'), key('c'), key('Tab')]);

      expect(textarea.value).toBe('ab');
      expect(keyRelay.release().map(k => k.key)).toEqual(['c', 'Tab']);
    });
  });
});
