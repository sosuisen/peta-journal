import { useEffect, type KeyboardEvent } from 'react';
import type { SearchEntry, SearchKey } from './border/peta-types';
import { useCore, usePanel, useSearchResults } from './context';
import { isImeComposing } from './dom';
import { useShownValue } from './shownValue';

/** How long the results stay after the input loses the focus: a press on one comes first. */
const CLOSE_AFTER_BLUR_MS = 200;

const KEYS: Readonly<Record<string, SearchKey['kind']>> = {
  ArrowDown: 'Down',
  ArrowUp: 'Up',
  Enter: 'Enter',
  Escape: 'Escape',
};

/** The search box of the header, ported from the Leptos shell. The core runs the search. */
export default function SearchBar() {
  const core = useCore();
  const search = usePanel('Search');
  const found = useSearchResults();
  const editing = usePanel('Outline')?.focused_item_id != null;
  // The query as the core has it, as cleared after a navigation
  const input = useShownValue(search?.query ?? '');

  useFocusKeys(input, editing);

  const pressed = (event: KeyboardEvent<HTMLInputElement>) => {
    if (isImeComposing(event.nativeEvent)) {
      return;
    }
    const kind = KEYS[event.key] as SearchKey['kind'] | undefined;
    if (kind === undefined) {
      return;
    }
    // While the answer is on its way, the core holds Enter until it arrives
    if (
      kind === 'Enter' &&
      search?.selected_index == null &&
      search?.searching !== true
    ) {
      return;
    }
    event.preventDefault();
    core.send({ kind: 'SearchKey', value: { kind } });
  };

  const results = search !== undefined && search.show_dropdown ? found : [];
  const selected = search?.selected_index ?? null;

  return (
    <app-search-bar>
      <div className="search-container">
        <input
          ref={input}
          type="text"
          className="search-input"
          placeholder="Type Ctrl+/ to search"
          onInput={event =>
            core.send({
              kind: 'SearchQueryChanged',
              value: event.currentTarget.value,
            })
          }
          onKeyDown={pressed}
          onFocus={() => core.send({ kind: 'SearchDropdown', value: true })}
          onBlur={() =>
            setTimeout(
              () => core.send({ kind: 'SearchDropdown', value: false }),
              CLOSE_AFTER_BLUR_MS,
            )
          }
        />
        {results.length > 0 && (
          <div className="search-dropdown">
            {results.map((entry, at) => (
              <Result
                key={entry.key}
                entry={entry}
                selected={selected === BigInt(at)}
                pressed={() =>
                  core.send({
                    kind: 'SearchResultClicked',
                    position: BigInt(at),
                  })
                }
              />
            ))}
          </div>
        )}
      </div>
    </app-search-bar>
  );
}

/**
 * Ctrl+/ focuses the search from anywhere. A bare / does so only outside the editors and the
 * inputs, where it would be a letter, and without Alt or Command.
 */
function useFocusKeys(
  input: React.RefObject<HTMLInputElement | null>,
  editing: boolean,
): void {
  useEffect(() => {
    const pressed = (event: globalThis.KeyboardEvent) => {
      if (event.key !== '/') {
        return;
      }
      if (!event.ctrlKey) {
        const target = event.target;
        if (
          editing ||
          event.altKey ||
          event.metaKey ||
          target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement
        ) {
          return;
        }
      }
      event.preventDefault();
      input.current?.focus();
    };
    document.addEventListener('keydown', pressed);
    return () => document.removeEventListener('keydown', pressed);
  }, [input, editing]);
}

/** One result. It answers the press, not the click: the blur of the input comes before. */
function Result({
  entry,
  selected,
  pressed,
}: {
  readonly entry: SearchEntry;
  readonly selected: boolean;
  readonly pressed: () => void;
}) {
  const mark = selected ? ' selected' : '';
  switch (entry.kind.kind) {
    case 'Today':
      return (
        <div className={`search-today${mark}`} onMouseDown={pressed}>
          {entry.label}
        </div>
      );
    case 'Create':
      return (
        <div className={`search-create${mark}`} onMouseDown={pressed}>
          {entry.label}
        </div>
      );
    case 'Page':
      return (
        <div className={`search-result${mark}`} onMouseDown={pressed}>
          <div className="result-page">
            <svg className="result-icon" width="14" height="14">
              <use href="#icon-page" />
            </svg>
            <span className="page-title">{entry.title}</span>
          </div>
        </div>
      );
    case 'Item':
      return (
        <div className={`search-result${mark}`} onMouseDown={pressed}>
          <div className="result-item">
            <svg className="result-icon" width="14" height="14">
              <use href="#icon-item" />
            </svg>
            <div className="item-content">
              <div className="item-page-path">{entry.title}</div>
              {/* The core marks the matches in the snippet (ADR 052) */}
              <div
                className="item-snippet"
                dangerouslySetInnerHTML={{ __html: entry.snippet }}
              />
            </div>
          </div>
        </div>
      );
  }
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-search-bar': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
