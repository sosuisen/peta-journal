import { describe, expect, test } from 'vitest';
import type { Asked } from './core';
import { Core } from './core';
import {
  AtlasCard,
  cardToneLink,
  Bounds,
  Callout,
  LinkCard,
  PropertyDefinition,
} from './border/peta-types';
import { WebShell } from './test/shell';
import { outline, row, shell } from './test/screens';

// wasmのコア
describe('the core in wasm', () => {
  // イベントは、wasmに渡る。
  test('an event goes to wasm', () => {
    const { core, wasm } = shell();

    core.send({ kind: 'EditFinished', item_id: 'a' });

    expect(wasm.events).toEqual([{ kind: 'EditFinished', item_id: 'a' }]);
  });

  // 呼び出しが返した差分は画面に当たり、画面を読む部品に知らされる。
  test('the changes a call answers go on the screen, and the readers are told', () => {
    const { core, wasm } = shell();
    let told = 0;
    core.subscribe(() => (told += 1));
    wasm.answerNext(
      [{ kind: 'Reset', items: [row('a', 'one')] }],
      [{ kind: 'Today', value: '2026-11-23' }],
    );

    core.typed({ kind: 'Changed', item_id: 'a', body: 'one' });

    expect(core.current().rows.map(r => r.item.body)).toEqual(['one']);
    expect(core.current().panels.Today).toBe('2026-11-23');
    expect(told).toBe(1);
  });

  // 起動すると、コアが開いたページの行が画面に出る。
  test('the start puts the rows of the page the core opened on the screen', () => {
    const { core, wasm } = shell();
    wasm.answerNext([{ kind: 'Reset', items: [row('a')] }]);

    core.start();

    expect(core.current().rows.map(r => r.item.id)).toEqual(['a']);
  });

  // サーバーの答えのように、呼び出しの外で変わった画面も、描き直しの知らせで読まれる。パネルだけの変化も読まれる。
  test('a screen changed outside a call is read when wasm asks for a render', () => {
    const { core, wasm } = shell();

    wasm.push([], [{ kind: 'Today', value: '2026-11-23' }]);

    expect(core.current().panels.Today).toBe('2026-11-23');
  });

  // コアが頼む画面の操作は、番号とともにJSONから読まれて、シェルに渡る。
  test('an operation the core asks for reaches the shell with its number', () => {
    const asked: Asked[] = [];
    new Core(op => asked.push(op));

    WebShell.last.ask(3, { kind: 'ConfirmDestructive', value: 'Delete?' });

    expect(asked).toEqual([
      { id: 3, op: { kind: 'ConfirmDestructive', value: 'Delete?' } },
    ]);
  });

  // 差分が空なら、画面は同じオブジェクトのままで、だれにも知らせない。
  test('without changes the screen is the very same, and no reader is told', () => {
    const { core } = shell();
    const before = core.current();
    let told = 0;
    core.subscribe(() => (told += 1));

    core.send({ kind: 'EditFinished', item_id: 'a' });

    expect(core.current()).toBe(before);
    expect(told).toBe(0);
  });

  // 読むのをやめた部品には、もう知らせない。
  test('a reader that stopped reading is not told', () => {
    const { core, wasm } = shell();
    let told = 0;
    const stop = core.subscribe(() => (told += 1));

    stop();
    wasm.push([], [{ kind: 'Outline', value: outline() }]);

    expect(told).toBe(0);
  });

  // コアが受け取らないキーとドラッグは、偽を返す。受け取れば、真を返して差分を当てる。
  test('a key or a drag the core does not take answers false', () => {
    const { core, wasm } = shell();
    const chord = {
      key: { kind: 'Enter' as const },
      shift: false,
      ctrl: false,
      alt: false,
      cmd: false,
    };

    wasm.takesKeys = false;
    wasm.takesDrags = false;
    const key = core.press({ kind: 'InSelection', chord });
    const drag = core.drag({ kind: 'Released' });
    wasm.takesKeys = true;
    wasm.takesDrags = true;
    wasm.answerNext([{ kind: 'Reset', items: [row('a')] }]);
    const taken = core.press({ kind: 'InSelection', chord });
    wasm.answerNext([{ kind: 'Insert', at: 1, item: row('b') }]);
    const dragged = core.drag({ kind: 'Released' });

    expect([key, drag, taken, dragged]).toEqual([false, false, true, true]);
    expect(core.current().rows.map(r => r.item.id)).toEqual(['a', 'b']);
  });

  // ほかの呼び出しも、渡したものをそのままwasmに渡す。
  test('the other calls hand what they are given to wasm', () => {
    const { core, wasm } = shell();
    const file = new File(['x'], 'image.png', { type: 'image/png' });

    core.start();
    core.pasteImage('a', file, 3);
    core.go('/pages/notes');
    core.resolve(7, { kind: 'Done' });
    core.beginCopy();

    expect(wasm.pastes).toEqual([{ itemId: 'a', file, cursor: 3 }]);
    expect(wasm.links).toEqual(['/pages/notes']);
    expect(wasm.answers).toEqual([{ id: 7, output: { kind: 'Done' } }]);
    expect(wasm.copies).toBe(1);
  });

  // 預けたキーの再生と、ウィキリンクの中の文字は、wasmが答える。
  test('wasm answers what a held key does and the text inside a wiki link', () => {
    const { core, wasm } = shell();
    wasm.replay = () => ({ kind: 'MoveTo', value: 2n });
    wasm.wikiQuery = (text, cursor) => text.slice(2, cursor);
    const key = {
      key: 'Home',
      code: 'Home',
      shift_key: false,
      ctrl_key: false,
      alt_key: false,
      meta_key: false,
    };

    const replay = core.replayOf(key, 'ab', 1, 2);
    const query = core.wikiQueryAt('[[no', 4);

    expect(replay).toEqual({ kind: 'MoveTo', value: 2n });
    expect(wasm.replays).toEqual([{ key, text: 'ab', start: 1, end: 2 }]);
    expect(query).toBe('no');
  });

  // タグのページのURLは、wasmが決める。
  test('wasm gives the url of the page of a tag', () => {
    const { core } = shell();

    expect(core.tagUrl('tags/', 'plan')).toBe('/pages/tags/plan');
  });

  // 設定に出すキーの名前と、新しいキーに選べる型は、wasmが決める。
  test('wasm gives the shown names of the keys and the types of a new key', () => {
    const { core } = shell();
    const definitions = ['rating_n', 'due_d'].map(
      key => new PropertyDefinition(key, 'text', false, false),
    );

    expect(core.shownNames(definitions)).toEqual([
      'shown rating_n',
      'shown due_d',
    ]);
    expect(core.userValueTypes()).toEqual(['text', 'number']);
  });

  // アトラスのキャンバスに描くカードと吹き出し、上のボタンの名前は、wasmが決める。
  test('wasm gives what the canvas of an atlas draws, and the name of the up button', () => {
    const { core } = shell();
    const card = new AtlasCard('a', 'A', 'm/a', false, null, [], [], false, '');
    const link = new LinkCard(
      'l',
      'x/Foo',
      'Foo',
      cardToneLink(),
      [],
      '',
      new Bounds(1, 2, 0, 0),
      [],
    );
    const callout = new Callout('k', 1, 2, 1, 'left', 'hi', []);

    expect(JSON.parse(core.atlasCards([card], [link]))).toEqual({
      cards: ['a'],
      links: ['l'],
    });
    expect(JSON.parse(core.atlasCallouts([callout]))).toEqual(['k']);
    expect(core.parentDirLabel('a/b/')).toBe('above a/b/');
  });

  // wasmは、ドラッグで項目を選ぶ間に、アウトラインの端でスクロールする幅と、その1歩の時間を決める。
  test('wasm gives how far and how often a drag scrolls the outline at an edge', () => {
    const { core } = shell();

    expect([
      core.edgeStep(105, 100, 300),
      core.edgeStep(200, 100, 300),
      core.edgeStep(295, 100, 300),
    ]).toEqual([-5, 0, 5]);
    expect(core.edgeStepMs()).toBe(16);
  });
});
