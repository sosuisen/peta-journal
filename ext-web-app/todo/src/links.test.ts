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
});
