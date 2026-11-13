import { expect, test } from 'vitest';
import { isOnLink } from './dom';

/** An element that has, or has not, a link around it or is one. */
function element(link: object | null) {
  return { closest: (selector: string) => (selector === 'a' ? link : null) };
}

// リンクの中の要素へのクリックは、リンクへのクリックである。
test('a click on an element inside a link is a click on a link', () => {
  expect(isOnLink(element({}))).toBe(true);
});

// リンクの外の要素へのクリックは、リンクへのクリックではない。
test('a click on an element outside a link is not a click on a link', () => {
  expect(isOnLink(element(null))).toBe(false);
});
