import type { ReactRenderOptions } from 'flatmarkdown-ast2react';

/**
 * Where the links in the text of a task lead: to the pages of the PetaJournal web app.
 * A hashtag leads to the page of the tag. The tag pages live in a folder of their own,
 * `tagsDir`, which is empty for the pages root and ends with `/` otherwise.
 */
export function linkOptions(
  serverUrl: string,
  tagsDir: string,
): ReactRenderOptions {
  const pages = `${serverUrl}/pages/`;
  return {
    wikilink: { hrefPrefix: pages },
    hashtag: { hrefPrefix: `${pages}${tagsDir}` },
  };
}
