import { useState } from 'react';
import FilterBar from './FilterBar';
import ItemRow from './ItemRow';
import { type Filters, groupByPlan, isShown, localISODate } from './tasks';
import { useTasks } from './useTasks';

/** The todo app: the tasks of the journal server, filtered and grouped by plan date. */
export default function App() {
  const { items, loading, error, connectionState, cycle } = useTasks();
  const [filters, setFilters] = useState<Filters>(() => ({
    untilDate: localISODate(new Date()),
    showDone: false,
    showJournal: true,
    showPage: false,
  }));

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
            <ItemRow key={entry.item.id} item={entry.item} onCycle={cycle} />
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
