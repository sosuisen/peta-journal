import hljs from 'highlight.js/lib/common';
import type { MouseEvent as ReactMouseEvent } from 'react';
import type { Held } from './border/peta-types';
import {
  eventLinkClicked,
  richLinkPage,
  richLinkWeb,
} from './border/peta-types';
import type { Core } from './core';

// What the shell does with a body that wasm drew as HTML (ADR 079): a row of a page, and the
// whole text of a card of an atlas.

/** The marks that the HTML of a body puts on a wikilink and on a hashtag. */
const PAGE_LINKS = 'a[data-wikilink], a.hashtag';

/** The keys held down at a click. */
export function heldOf(event: ReactMouseEvent): Held {
  return { shift: event.shiftKey, ctrl: event.ctrlKey, cmd: event.metaKey };
}

/**
 * Opens what a click in a drawn body names. A footnote comes to the middle of the screen. A
 * link goes to the core with the keys `held`, and the core decides how it opens (ADR 083).
 */
export function clickedLink(
  core: Core,
  event: ReactMouseEvent<HTMLDivElement>,
  held: Held,
): void {
  // A click lands on an element, never on a text node
  const target = event.target as Element;
  const footnote = target.closest('.footnote-ref a, .footnote-label');
  if (footnote !== null) {
    event.preventDefault();
    event.stopPropagation();
    // flatmarkdown gives both links of a footnote an href of the form #id
    const href = footnote.getAttribute('href') as string;
    document.getElementById(href.slice(1))?.scrollIntoView({ block: 'center' });
    return;
  }
  const page = target.closest(PAGE_LINKS);
  const href = page?.getAttribute('href') ?? null;
  // The address the browser reads from the link, also for a file of the assets folder
  const link = page === null ? target.closest('a') : null;
  const clicked =
    href !== null
      ? richLinkPage(href)
      : link !== null && link.href !== ''
        ? richLinkWeb(link.href)
        : null;
  if (clicked === null) {
    return;
  }
  // The browser would open it too
  event.preventDefault();
  event.stopPropagation();
  core.send(eventLinkClicked(clicked, held));
}

/**
 * Gives each image the size that the fragment of its URL carries (`#w=300&h=240`). The wasm
 * writes only the sides that the core reads as pixels, so they are not checked again here.
 */
export function sizeImages(content: Element): void {
  for (const image of content.querySelectorAll('img')) {
    // A URL without a fragment gives no sides
    const sides = new URLSearchParams(image.getAttribute('src')?.split('#')[1]);
    for (const [side, name] of [
      ['w', 'width'],
      ['h', 'height'],
    ] as const) {
      const pixels = sides.get(side);
      if (pixels !== null) {
        image.setAttribute(name, pixels);
      }
    }
  }
}

/**
 * Colours each code block that names its language, and shows the name on it. A block
 * without a language is left alone: highlight.js would guess one, and colour plain text.
 */
export function highlightCode(content: Element): void {
  for (const code of content.querySelectorAll<HTMLElement>('pre code')) {
    const language = [...code.classList]
      .find(name => name.startsWith('language-'))
      ?.slice('language-'.length);
    const pre = code.parentElement;
    if (language === undefined || pre === null) {
      continue;
    }
    hljs.highlightElement(code);
    if (pre.querySelector('.code-lang-label') === null) {
      const label = document.createElement('div');
      label.className = 'code-lang-label';
      label.textContent = language;
      pre.style.position = 'relative';
      pre.append(label);
    }
  }
}
