import type { ReactRenderOptions } from 'flatmarkdown-ast2react';

/** Where the links in the text of a task lead: to the pages of the PetaJournal web app. */
export function linkOptions(
  serverUrl: string,
  tagsDir: string,
): ReactRenderOptions {
  return {
    wikilink: { hrefPrefix: `${serverUrl}/pages/` },
  };
}
