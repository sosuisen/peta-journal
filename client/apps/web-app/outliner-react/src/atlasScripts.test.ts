import { afterEach, describe, expect, test } from 'vitest';
import { ATLAS_SCRIPTS, scriptsLoader } from './atlasScripts';

afterEach(() => {
  document.head.replaceChildren();
});

const added = () =>
  [...document.head.querySelectorAll('script')].map(s => s.getAttribute('src'));

const loaded = (n: number) =>
  document.head.querySelectorAll('script')[n].dispatchEvent(new Event('load'));

// アトラスのスクリプト
describe('the scripts of the atlas', () => {
  // スクリプトは、PixiJSとatlas.jsの順である。atlas.jsは、PixiJSがグローバルに置くものを使う。
  test('the scripts are PixiJS and atlas.js, in this order', () => {
    expect(
      ATLAS_SCRIPTS.map(url => url.split('/').pop()?.split('?')[0]),
    ).toEqual(['pixi.min.js', 'atlas.js']);
  });

  // 1つずつ順に読み、前のものを読み終えてから次を足す。すべて読み終えたら、約束が果たされる。
  test('each script is added once the one before it is loaded, and all of them end the promise', async () => {
    const load = scriptsLoader(['a.js', 'b.js']);

    const done = load();
    const first = added();
    loaded(0);
    await Promise.resolve();
    const second = added();
    loaded(1);
    await done;

    expect(first).toEqual(['a.js']);
    expect(second).toEqual(['a.js', 'b.js']);
  });

  // 2度目からは、同じ約束を返し、スクリプトを足し直さない。
  test('the scripts are loaded only once', () => {
    const load = scriptsLoader(['a.js']);

    const promises = [load(), load()];

    expect(promises[0]).toBe(promises[1]);
    expect(added()).toEqual(['a.js']);
  });

  // 読めないスクリプトがあれば、約束は破られ、その先は読まない。
  test('a script that fails breaks the promise, and the rest are not loaded', async () => {
    const load = scriptsLoader(['a.js', 'b.js']);

    const done = load();
    document.head.querySelector('script')?.dispatchEvent(new Event('error'));

    await expect(done).rejects.toThrow('a.js');
    expect(added()).toEqual(['a.js']);
  });
});
