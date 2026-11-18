import type { ReactRenderOptions } from 'flatmarkdown-ast2react';
import type { LinkConfig } from './api';

/**
 * Where the links in the text of a task lead: to the pages of the PetaJournal web app. The
 * server gives the templates of the links, so that every app makes the same URLs (ADR 061).
 *
 * The templates start at the root of the server. This app runs at another address than the
 * pages, so the address of the server goes before them. An image is written with a path
 * relative to its page, such as `../assets/pic.png`, so it is resolved against the pages.
 */
export function linkOptions(
  serverUrl: string,
  links: LinkConfig,
): ReactRenderOptions {
  return {
    wikilink: {
      href: `${serverUrl}${links.wikilinkHref}`,
      fragmentPattern: links.wikilinkFragmentPattern,
    },
    hashtag: { href: `${serverUrl}${links.hashtagHref}` },
    embed: { baseUrl: `${serverUrl}${links.pagesBase}` },
  };
}
