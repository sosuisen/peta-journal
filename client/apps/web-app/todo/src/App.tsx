import { useMemo, useState } from 'react';
import { serverUrl } from './api';
import FilterBar from './FilterBar';
import ItemRow from './ItemRow';
import { linkOptions } from './links';
import { type Filters, groupByPlan, isShown, localISODate } from './tasks';
import { useTasks } from './useTasks';

/** The todo app: the tasks of the journal server, filtered and grouped by plan date. */
export default function App() {
  const [filters, setFilters] = useState<Filters>(() => ({
    untilDate: localISODate(new Date()),
    showDone: false,
    showJournal: true,
    showPage: false,
  }));
  const { items, loading, error, connectionState, links, cycle } = useTasks(
    filters.showDone,
  );
  // FmRoot renders again when the options are a new object, so they are kept between renders.
  // Until the server has given the templates, the links lead nowhere
  const itemLinkOptions = useMemo(
    () => (links ? linkOptions(serverUrl, links) : {}),
    [links],
  );

  const entries = groupByPlan(items.filter(item => isShown(item, filters)));

  return (
    <div className="app">
      <h1>PetaApps ToDo</h1>

      {error && <div className="error">{error}</div>}

      <FilterBar filters={filters} onChange={setFilters} />

      <div className="item-list">
        {entries.length === 0 && !loading && (
          <div className="empty">No items found</div>
        )}
        {entries.map(entry =>
          entry.kind === 'heading' ? (
            <div key={`h-${entry.label}`} className="plan-heading">
              {entry.label}
            </div>
          ) : (
            <ItemRow
              key={entry.item.id}
              item={entry.item}
              linkOptions={itemLinkOptions}
              onCycle={item => void cycle(item)}
            />
          ),
        )}
      </div>
      <footer className="footer">
        <span className={`connection-status ${connectionState}`}>
          {connectionState}
        </span>
      </footer>
    </div>
  );
}
