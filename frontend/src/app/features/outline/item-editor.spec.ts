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
  });
});
