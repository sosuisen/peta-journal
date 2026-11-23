import { For, Show } from 'solid-js';
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
 * panel after each letter: the panel is replaced whole while the user types.
 */
export default function SearchBar(props: { core: Core; search: SearchView }) {
  return (
    <div class="search">
      <input
        type="search"
        placeholder="検索"
        value={props.search.query}
        onInput={event => {
          measure('key');
          props.core.send({ SearchQueryChanged: event.currentTarget.value });
        }}
        onKeyDown={event => {
          const key = SEARCH_KEYS[event.key];
          if (key !== undefined && !event.isComposing) {
            event.preventDefault();
            props.core.send({ SearchKey: key });
          }
        }}
        onFocus={() => props.core.send({ SearchDropdown: true })}
        onBlur={() => props.core.send({ SearchDropdown: false })}
      />
      <Show when={props.search.show_dropdown && props.search.results.length > 0}>
        <ul>
          <For each={props.search.results} keyed={entry => entry.position}>
            {entry => (
              <li
                class={entry().position === props.search.selected_index ? 'chosen' : undefined}
                // Before the blur of the field, which closes the list
                onMouseDown={event => {
                  event.preventDefault();
                  props.core.send({ SearchResultClicked: { position: entry().position } });
                }}
              >
                <div>{entry().label}</div>
                <Show when={entry().snippet !== ''}>
                  <div class="snippet">{entry().snippet}</div>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
