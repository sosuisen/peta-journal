import { describe, expect, test } from 'vitest';
import { linkOptions } from './links';

// 本文のリンクの行き先
describe('link options', () => {
  // ウィキリンクは、PetaJournalのWebアプリのページへ行く。
  test('a wikilink leads to a page of the web app', () => {
    const options = linkOptions('http://journal.test:9000', '');

    expect(options.wikilink?.hrefPrefix).toBe(
      'http://journal.test:9000/pages/',
    );
  });

  // ハッシュタグは、タグのフォルダにあるページへ行く。
  test('a hashtag leads to a page in the folder of the tag pages', () => {
    const options = linkOptions('http://journal.test:9000', 'tags/');

    expect(options.hashtag?.hrefPrefix).toBe(
      'http://journal.test:9000/pages/tags/',
    );
  });

  // 画像は、ページからの相対パスで書かれている。Webアプリのページのアドレスから解決する。
  test('an image is resolved against the address of the pages', () => {
    const options = linkOptions('http://journal.test:9000', '');

    expect(options.embed?.baseUrl).toBe('http://journal.test:9000/pages/');
  });
});
