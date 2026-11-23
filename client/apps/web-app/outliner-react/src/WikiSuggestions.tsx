import { WikiLinkContext } from './border/peta-types';
import { useCore, usePanel } from './context';
import type { Core } from './core';
import { caretRect } from './dom';

/**
 * Tells the core the `[[wiki link]]` the cursor of the editor is in, with the place of the
 * cursor on the screen, or that it is in none. The core searches the page names and closes
 * the popup.
 */
export function checkWikiLink(
  core: Core,
  itemId: string,
  el: HTMLTextAreaElement,
): void {
  const start = el.selectionStart;
  const query = core.wikiQueryAt(el.value, start);
  let context: WikiLinkContext | null = null;
  if (query !== undefined) {
    const { x, bottom } = caretRect(el, start);
    context = new WikiLinkContext(query, x, bottom);
  }
  core.send({ kind: 'WikiQuery', item_id: itemId, context });
}

/** The popup with the page names that match the text typed inside a `[[wiki link]]`. */
export default function WikiSuggestions() {
  const core = useCore();
  const wiki = usePanel('Outline')?.wiki;
  if (wiki === undefined || wiki.suggestions.length === 0) {
    return null;
  }
  const selected = Number(wiki.selected_index);
  return (
    <app-wiki-suggestions>
      <div
        className="wiki-suggestions"
        style={{ left: `${wiki.x}px`, top: `${wiki.y}px` }}
      >
        {wiki.suggestions.map((title, i) => (
          <div
            key={`${i}:${title}`}
            className={
              i === selected
                ? 'wiki-suggestion-item selected'
                : 'wiki-suggestion-item'
            }
            onMouseDown={event => {
              // The editor keeps the focus: a blur would end the edit
              event.preventDefault();
              event.stopPropagation();
              core.send({ kind: 'WikiPick', title });
            }}
          >
            {title}
          </div>
        ))}
      </div>
    </app-wiki-suggestions>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-wiki-suggestions': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
