import type { Core } from '../../shared/src/core';
import { measure } from '../../shared/src/metrics';
import type { SearchView } from '../../shared/src/types';

const SEARCH_KEYS: Readonly<Record<string, string>> = {
  ArrowDown: 'Down',
  ArrowUp: 'Up',
  Enter: 'Enter',
  Escape: 'Escape',
};

/**
 * The search field. Its value is the query of the core, which comes back in the `Search`
 * panel after each letter: the panel is drawn again whole while the user types.
 */
export default function SearchBar({ core, search }: { core: Core; search: SearchView }) {
  return (
    <div className="search">
      <input
        type="search"
        placeholder="検索"
        value={search.query}
        onChange={event => {
          measure('key');
          core.send({ SearchQueryChanged: event.currentTarget.value });
        }}
        onKeyDown={event => {
          const key = SEARCH_KEYS[event.key];
          if (key !== undefined && !event.nativeEvent.isComposing) {
            event.preventDefault();
            core.send({ SearchKey: key });
          }
        }}
        onFocus={() => core.send({ SearchDropdown: true })}
        onBlur={() => core.send({ SearchDropdown: false })}
      />
      {search.show_dropdown && search.results.length > 0 && (
        <ul>
          {search.results.map(entry => (
            <li
              key={entry.position}
              className={entry.position === search.selected_index ? 'chosen' : undefined}
              // Before the blur of the field, which closes the list
              onMouseDown={event => {
                event.preventDefault();
                core.send({ SearchResultClicked: { position: entry.position } });
              }}
            >
              <div>{entry.label}</div>
              {entry.snippet !== '' && <div className="snippet">{entry.snippet}</div>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
