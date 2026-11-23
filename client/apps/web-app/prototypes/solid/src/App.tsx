import { createSignal, createStore, onSettled } from 'solid-js';
import type { Core } from '../../shared/src/core';
import { applyInPlace } from '../../shared/src/list';
import { measureUntil } from '../../shared/src/metrics';
import { installProbe } from '../../shared/src/probe';
import type { ListChange, OutlinePanel, Row, SearchView } from '../../shared/src/types';
import Outline, { type OutlineHandle } from './Outline';
import SearchBar from './SearchBar';

const NO_SEARCH: SearchView = {
  query: '',
  results: [],
  show_dropdown: false,
  selected_index: null,
  searching: false,
};

/**
 * How the rows take the changes: in a store, with a `splice` for each change, or as a new
 * plain array in a signal, as the React prototype does. `?apply=signal` picks the second.
 */
function rowsOf(apply: string | null): {
  rows: () => readonly Row[];
  apply: (changes: readonly ListChange[]) => void;
} {
  if (apply === 'signal') {
    const [rows, setRows] = createSignal<readonly Row[]>([]);
    return {
      rows,
      apply: changes =>
        setRows(had => {
          const next = [...had];
          applyInPlace(next, changes);
          return next;
        }),
    };
  }
  const [rows, setRows] = createStore<Row[]>([]);
  return { rows: () => rows, apply: changes => setRows(draft => applyInPlace(draft, changes)) };
}

export default function App(props: { core: Core }) {
  const core = props.core;
  const applyBy =
    new URLSearchParams(location.search).get('apply') === 'signal' ? 'signal' : 'store';
  const { rows, apply } = rowsOf(applyBy);
  const [outline, setOutline] = createSignal<OutlinePanel | null>(null);
  const [search, setSearch] = createSignal<SearchView>(NO_SEARCH);
  let handle: OutlineHandle | undefined;

  onSettled(() => {
    installProbe(`Solid (${applyBy})`, core, rows);
    measureUntil('open', () => rows().length > 0);
    core.start({
      rows: apply,
      outline: panel => setOutline(panel),
      search: view => setSearch(view),
      focus: itemId => handle?.focus(itemId),
      scrollTo: itemId => handle?.scrollTo(itemId),
    });
  });

  return (
    <div class="app">
      <div class="toolbar">
        <span class="framework">Solid</span>
        <SearchBar core={core} search={search()} />
        <span class="numbers">
          {rows().length}行・{core.carry}・{applyBy}
        </span>
      </div>
      <Outline
        core={core}
        rows={rows()}
        outline={outline()}
        handle={value => {
          handle = value;
        }}
      />
    </div>
  );
}
