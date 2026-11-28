// The judgements of atlas.js that need no PixiJS: what a drop does, and which groups a drag can
// take what it moves out of. atlas.js is a plain script that the pages load as it is, so it is
// run here as a script too, and its functions are read from its globals. Nothing of PixiJS runs
// while it loads: only `window` is needed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const atlas = vm.createContext({ window: {} });
vm.runInContext(readFileSync(new URL('./atlas.js', import.meta.url), 'utf8'), atlas);

// The values of atlas.js are made in its own realm: plain copies compare as values of this one
const plain = value => JSON.parse(JSON.stringify(value));
const sorted = set => [...set].sort();

const piece = (id, group) => ({ id, group, x: 0, y: 0 });

// ドロップ
// Ctrlでカードに放すと、そのカードの兄弟になる
test('a drop with Ctrl on a card makes the pieces siblings of that card', () => {
  const action = atlas.dropAction({ mode: 'sibling', target: 'card', join: null, pieces: [piece('a', null)] });
  assert.deepEqual(plain(action), { kind: 'group', target: 'card' });
});

// Shift+Ctrlでカードに放すと、そのカードの子になる
test('a drop with Shift and Ctrl on a card puts the pieces under that card', () => {
  const action = atlas.dropAction({ mode: 'child', target: 'card', join: null, pieces: [piece('a', null)] });
  assert.deepEqual(plain(action), { kind: 'move', parent: 'card' });
});

// Ctrlでほかのグループの領域に放すと、そのグループの項目の下に入る
test('a drop with Ctrl in the area of another group puts the pieces under its item', () => {
  const action = atlas.dropAction({ mode: 'sibling', target: null, join: 'other', pieces: [piece('a', 'g')] });
  assert.deepEqual(plain(action), { kind: 'move', parent: 'other' });
});

// Ctrlで自分のグループの外に放したものだけが、そのグループを出る。ほかは置かれるだけである
test('a drop with Ctrl takes out only the pieces let go outside their group', () => {
  const action = atlas.dropAction({
    mode: 'sibling',
    outside: new Set(['left']),
    pieces: [piece('a', 'left'), piece('b', 'kept'), piece('c', null)],
  });
  assert.deepEqual(
    plain({ kind: action.kind, place: action.place.map(p => p.id), leave: action.leave.map(p => p.id) }),
    { kind: 'place', place: ['b', 'c'], leave: ['a'] },
  );
});

// キーなしや、Shift+Ctrlで何もない所に放すと、グループの外でも置かれるだけである
test('a drop without Ctrl alone only places the pieces, outside their group too', () => {
  for (const mode of [null, 'child']) {
    const action = atlas.dropAction({ mode, outside: new Set(['g']), pieces: [piece('a', 'g')] });
    assert.deepEqual(
      plain({ kind: action.kind, place: action.place.map(p => p.id), leave: action.leave.length }),
      { kind: 'place', place: ['a'], leave: 0 },
      String(mode),
    );
  }
});

// Ctrlで出られるグループ
// ドラッグしたカードは、自分の入っているグループを、入れ子のどれも出られる。ほかのグループは出られない
test('a drag with Ctrl can leave each group its cards are in, however deep, and no other', () => {
  const held = atlas.heldGroups({ dragged: [['outer', 'inner'], ['outer']] });
  assert.deepEqual(sorted(held), ['inner', 'outer']);
});

// グループのただ1つの子をドラッグしても、そのグループを出られる
test('a drag with Ctrl can leave a group whose only card it drags', () => {
  const held = atlas.heldGroups({ dragged: [['solo']] });
  assert.deepEqual(sorted(held), ['solo']);
});

// グループのすべての子をドラッグしても、そのグループを出られる
test('a drag with Ctrl can leave a group all of whose cards it drags', () => {
  const held = atlas.heldGroups({ dragged: [['pair'], ['pair']] });
  assert.deepEqual(sorted(held), ['pair']);
});

// ドラッグしたグループは、その周りのグループを、どれも出られる
test('a dragged group can leave each group around it with Ctrl', () => {
  const held = atlas.heldGroups({ around: ['inner', 'outer'] });
  assert.deepEqual(sorted(held), ['inner', 'outer']);
});

// カードとキャンバスのメニュー
// ページのカードのメニューは、EditとRemove from Atlasである。項目を消すDelete Cardはない
test('the menu of a page card has Edit and Remove from Atlas', () => {
  assert.deepEqual(plain(atlas.cardMenu({ isLink: false, isPlain: false }, true)), ['Edit', 'Remove from Atlas']);
});

// リンクカードとノーマルカードのメニューは、Edit、Remove from Atlas、Delete Cardである
test('the menu of a link card or a plain card has Edit, Remove from Atlas and Delete Card', () => {
  for (const isPlain of [false, true]) {
    assert.deepEqual(plain(atlas.cardMenu({ isLink: true, isPlain }, true)), ['Edit', 'Remove from Atlas', 'Delete Card']);
  }
});

// 書き換えられないキャンバスでは、どのカードのメニューもView Textだけである
test('on a canvas that is not editable, the menu of any card has View Text alone', () => {
  for (const data of [{ isLink: false }, { isLink: true, isPlain: false }, { isLink: true, isPlain: true }]) {
    assert.deepEqual(plain(atlas.cardMenu(data, false)), ['View Text']);
  }
});

// キャンバスの何もない所のメニューは、Add CardとAdd Calloutである
test('the menu of the canvas has Add Card and Add Callout', () => {
  assert.deepEqual(plain(atlas.canvasMenu()), ['Add Card', 'Add Callout']);
});

// テキストのダイアログ
// シェルに開いてもらうダイアログは、名前と本文と辺を持つ。カードには辺がない
test('the dialog asked of the shell has the name, the text, and no side for a card', () => {
  assert.deepEqual(plain(atlas.textAsked({ name: 'Add Card', sided: false })), {
    name: 'Add Card',
    id: null,
    text: '',
    side: null,
    mode: 'edit',
  });
});

// View Textのダイアログは、見るだけで、カードの id を持つ。辺はない
test('the dialog of View Text is to view only, names its card and has no side', () => {
  assert.deepEqual(
    plain(atlas.textAsked({ name: 'View Text', id: 'c', text: 'read', side: 'left', sided: true, view: true })),
    { name: 'View Text', id: 'c', text: 'read', side: null, mode: 'view' },
  );
});

// グループと吹き出しには、今の辺がある。辺のないものや、知らない辺は、上である
test('a group or a callout asks with its side, the top when it has none it knows', () => {
  const sides = [undefined, 'sideways', 'right'].map(side =>
    plain(atlas.textAsked({ name: 'Edit', text: 'Plan', side, sided: true })).side,
  );
  assert.deepEqual(sides, ['top', 'top', 'right']);
});

// 円で描くグループ
const card =(...groups) => ({ groups, callout: false });
const callout = (...groups) => ({ groups, callout: true });

// 子がコールアウトだけのグループは、円で描く
test('a group whose children are only callouts is drawn as a circle', () => {
  const circles = atlas.circleGroups([callout('g'), callout('g'), card('other')]);
  assert.deepEqual(sorted(circles), ['g']);
});

// カードかグループを子に持つグループは、コールアウトの子があっても、円で描かない
test('a group with a card or a group among its children is not drawn as a circle', () => {
  const circles = atlas.circleGroups([
    callout('mixed'),
    card('mixed'),
    callout('outer'),
    callout('outer', 'inner'),
    card('deep', 'carded'),
  ]);
  assert.deepEqual(sorted(circles), ['inner']);
});
