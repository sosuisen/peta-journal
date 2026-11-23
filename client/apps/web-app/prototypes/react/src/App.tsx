import { useEffect, useRef, useState } from 'react';
import type { Core } from '../../shared/src/core';
import { applyInPlace } from '../../shared/src/list';
import { measureUntil } from '../../shared/src/metrics';
import { installProbe } from '../../shared/src/probe';
import type { OutlinePanel, Row, SearchView } from '../../shared/src/types';
import Outline, { type OutlineHandle } from './Outline';
import SearchBar from './SearchBar';

const NO_SEARCH: SearchView = {
  query: '',
  results: [],
  show_dropdown: false,
  selected_index: null,
  searching: false,
};

export default function App({ core }: { core: Core }) {
  const [rows, setRows] = useState<readonly Row[]>([]);
  const [outline, setOutline] = useState<OutlinePanel | null>(null);
  const [search, setSearch] = useState<SearchView>(NO_SEARCH);
  const outlineRef = useRef<OutlineHandle>(null);
  const shown = useRef<readonly Row[]>([]);

  useEffect(() => {
    installProbe('React', core, () => shown.current);
    measureUntil('open', () => shown.current.length > 0);
    core.start({
      rows: changes =>
        setRows(had => {
          const next = [...had];
          applyInPlace(next, changes);
          shown.current = next;
          return next;
        }),
      outline: setOutline,
      search: setSearch,
      focus: itemId => outlineRef.current?.focus(itemId),
      scrollTo: itemId => outlineRef.current?.scrollTo(itemId),
    });
  }, [core]);

  return (
    <div className="app">
      <div className="toolbar">
        <span className="framework">React</span>
        <SearchBar core={core} search={search} />
        <span className="numbers">
          {rows.length}行・{core.carry}
        </span>
      </div>
      <Outline ref={outlineRef} core={core} rows={rows} outline={outline} />
    </div>
  );
}
