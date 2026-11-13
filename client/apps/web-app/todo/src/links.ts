import type { ReactRenderOptions } from 'flatmarkdown-ast2react';

/**
 * Where the links in the text of a task lead: to the pages of the PetaJournal web app.
 * A hashtag leads to the page of the tag. The tag pages live in a folder of their own,
 * `tagsDir`, which is empty for the pages root and ends with `/` otherwise.
 *
 * An image is written with a path relative to its page, such as `../assets/pic.png`. This
 * app runs at another address than the pages, so the path is resolved against theirs.
 */
export function linkOptions(
  serverUrl: string,
  tagsDir: string,
): ReactRenderOptions {
  const pages = `${serverUrl}/pages/`;
  return {
    wikilink: { hrefPrefix: pages },
    hashtag: { hrefPrefix: `${pages}${tagsDir}` },
    embed: { baseUrl: pages },
  };
}
