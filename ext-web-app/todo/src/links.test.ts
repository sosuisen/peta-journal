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
});
