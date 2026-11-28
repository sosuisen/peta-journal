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
// ほかのカードが残るグループは、ドラッグしたカードが出られる。入れ子のグループは、どれも出られる
test('a drag with Ctrl can leave each group of its cards that other cards stay in', () => {
  const held = atlas.heldGroups({
    dragged: [['outer', 'inner']],
    staying: [['outer', 'inner'], ['outer'], ['elsewhere']],
  });
  assert.deepEqual(sorted(held), ['inner', 'outer']);
});

// グループのただ1つの子をドラッグしても、そのグループを出られる
test('a drag with Ctrl can leave a group whose only card it drags', () => {
  const held = atlas.heldGroups({ dragged: [['solo']], staying: [['elsewhere']] });
  assert.deepEqual(sorted(held), ['solo']);
});

// グループのすべての子をドラッグしても、そのグループを出られる
test('a drag with Ctrl can leave a group all of whose cards it drags', () => {
  const held = atlas.heldGroups({ dragged: [['pair'], ['pair']], staying: [] });
  assert.deepEqual(sorted(held), ['pair']);
});

// ドラッグしたグループは、その周りのグループを、どれも出られる
test('a dragged group can leave each group around it with Ctrl', () => {
  const held = atlas.heldGroups({ around: ['inner', 'outer'] });
  assert.deepEqual(sorted(held), ['inner', 'outer']);
});
