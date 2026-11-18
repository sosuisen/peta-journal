import { describe, expect, test } from 'vitest';
import { linkOptions } from './links';

/** The values for links, as the server gives them in its configuration. */
const config = {
  wikilinkHref: '/pages/{}',
  hashtagHref: '/pages/tags/{}',
  pagesBase: '/pages/',
  wikilinkFragmentPattern: '[0-9]{14}_[0-9A-Z]{7}',
};

// 本文のリンクの行き先
describe('link options', () => {
  // ウィキリンクは、サーバーのテンプレートのとおり、PetaJournalのWebアプリのページへ行く。項目のidの形も、サーバーから取る。
  test('a wikilink leads to a page of the web app by the template of the server', () => {
    const options = linkOptions('http://journal.test:9000', config);

    expect(options.wikilink?.href).toBe('http://journal.test:9000/pages/{}');
    expect(options.wikilink?.fragmentPattern).toBe('[0-9]{14}_[0-9A-Z]{7}');
  });

  // ハッシュタグは、サーバーのテンプレートのとおり、タグのフォルダにあるページへ行く。
  test('a hashtag leads to a page in the folder of the tag pages', () => {
    const options = linkOptions('http://journal.test:9000', config);

    expect(options.hashtag?.href).toBe('http://journal.test:9000/pages/tags/{}');
  });

  // 画像は、ページからの相対パスで書かれている。Webアプリのページのアドレスから解決する。
  test('an image is resolved against the address of the pages', () => {
    const options = linkOptions('http://journal.test:9000', config);

    expect(options.embed?.baseUrl).toBe('http://journal.test:9000/pages/');
  });
});
