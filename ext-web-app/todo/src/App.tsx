import { useCallback, useEffect, useRef, useState } from 'react';
import {
  buildBaseUrl,
  connect,
  onConnectionStateChange,
  onServerEvent,
  getItemsByTypes,
  cycleTask,
  type ConnectionState,
  type Item,
} from './api';
import { FmRoot, type ReactRenderOptions } from 'flatmarkdown-ast2react';

/** Returns today's date as an ISO string (YYYY-MM-DD). */
function getTodayISODate(): string {
  return new Date().toISOString().slice(0, 10);
}

function taskLabel(item: Item): string {
  if (item.properties?.end) return 'DONE';
  if (item.properties?.begin) return 'RUNNING';
  return 'TODO';
}

/** Gets the plan date from an item's properties, or null if absent. */
function getPlan(item: Item): string | null {
  const plan = item.properties?.plan;
  return typeof plan === 'string' ? plan : null;
}

/** Checks if an item belongs to a journal page (filename matches YYYY-MM-DD). */
function isJournal(item: Item): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(item.name || '');
}

type DisplayEntry =
  { kind: 'heading'; label: string } | { kind: 'item'; item: Item };

/** Groups items by plan date, with no-plan items at the end under a heading. */
function groupByPlan(items: Item[]): DisplayEntry[] {
  const withPlan: Item[] = [];
  const noPlan: Item[] = [];

  for (const item of items) {
    if (getPlan(item)) {
      withPlan.push(item);
    } else {
      noPlan.push(item);
    }
  }

  // Sort items with plan by plan date ascending
  withPlan.sort((a, b) => getPlan(a)!.localeCompare(getPlan(b)!));

  const entries: DisplayEntry[] = [];

  // Group with-plan items by date, adding date headings
  let currentDate = '';
  for (const item of withPlan) {
    const plan = getPlan(item)!;
    if (plan !== currentDate) {
      currentDate = plan;
      entries.push({ kind: 'heading', label: plan });
    }
    entries.push({ kind: 'item', item });
  }

  // Append no-plan items
  if (noPlan.length > 0) {
    entries.push({ kind: 'heading', label: '開始日なし' });
    for (const item of noPlan) {
      entries.push({ kind: 'item', item });
    }
  }

  return entries;
}

function ItemDates({ item }: { item: Item }) {
  const plan =
    typeof item.properties?.plan === 'string' ? item.properties.plan : null;
  const due =
    typeof item.properties?.due === 'string' ? item.properties.due : null;
  if (!plan && !due) return null;
  return (
    <span className="item-dates">
      {plan && <span>開始: {plan}</span>}
      {due && <span>期限: {due}</span>}
    </span>
  );
}

/** Main todo application component. Displays task items with filtering and sorting. */
const fmOptions: ReactRenderOptions = {
  wikilink: { hrefPrefix: `${buildBaseUrl()}/pages/` },
};

export default function App() {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('disconnected');
  const [error, setError] = useState('');
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);

  // Filters
  const [untilDate, setUntilDate] = useState(getTodayISODate());
  const [showDone, setShowDone] = useState(false);
  const [showJournal, setShowJournal] = useState(true);
  const [showPage, setShowPage] = useState(false);

  const connectedRef = useRef(false);

  const fetchItems = useCallback(async () => {
    if (!connectedRef.current) return;
    setLoading(true);
    try {
      const result = await getItemsByTypes(['task']);
      setItems(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const retryTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const scheduleRetry = useCallback(() => {
    if (retryTimer.current) return; // already scheduled
    retryTimer.current = setTimeout(() => {
      retryTimer.current = null;
      connectWithRetry();
    }, 3000);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const connectWithRetry = useCallback(() => {
    setError('');
    connect()
      .then(() => {
        connectedRef.current = true;
        setError('');
        fetchItems();
      })
      .catch(() => {
        connectedRef.current = false;
        scheduleRetry();
      });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onConnectionStateChange(state => {
      setConnectionState(state);
      if (state === 'disconnected' || state === 'error') {
        connectedRef.current = false;
        scheduleRetry();
      }
    });
    connectWithRetry();
    return () => {
      if (retryTimer.current) clearTimeout(retryTimer.current);
    };
  }, [connectWithRetry]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onServerEvent(() => {
      fetchItems();
    });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const openItemInPetaJournal = (item: Item) => {
    const name = item.name;
    if (!name) return;
    const isJournal = /^\d{4}(-\d{2}(-\d{2})?|-W\d{1,2})?$/.test(name);
    const prefix = isJournal ? '/journals' : '/docs';
    const path = `${prefix}/${item.dirPath || ''}${name}`;
    window.open(`${buildBaseUrl()}${path}#${item.id}`, 'peta-journal');
  };

  const cycleItemTask = async (item: Item) => {
    try {
      const updated = await cycleTask(item.id!);
      setItems(prev => prev.map(b => (b.id === updated.id ? updated : b)));
    } catch (e) {
      setError(String(e));
    }
  };

  const filtered = items.filter(b => {
    if (!showDone && b.properties?.end) return false;
    if (isJournal(b)) {
      if (!showJournal) return false;
    } else {
      if (!showPage) return false;
    }
    // Filter by plan date: items with plan <= untilDate, items without plan always shown
    if (untilDate) {
      const plan = getPlan(b);
      if (plan && plan > untilDate) return false;
    }
    return true;
  });

  const entries = groupByPlan(filtered);

  return (
    <div className="app">
      <h1>PetaApps ToDo</h1>

      {error && <div className="error">{error}</div>}

      <div className="filter-bar">
        <label>
          開始日:
          <input
            type="date"
            value={untilDate}
            onChange={e => setUntilDate(e.target.value)}
          />
        </label>

        <label className="checkbox">
          <input
            type="checkbox"
            checked={showDone}
            onChange={e => setShowDone(e.target.checked)}
          />
          Done
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={showJournal}
            onChange={e => setShowJournal(e.target.checked)}
          />
          Journal
        </label>
        <label className="checkbox">
          <input
            type="checkbox"
            checked={showPage}
            onChange={e => setShowPage(e.target.checked)}
          />
          Page
        </label>
      </div>

      <div className="item-list">
        {entries.length === 0 && !loading && (
          <div className="empty">No items found</div>
        )}
        {entries.map((entry, i) =>
          entry.kind === 'heading' ? (
            <div key={`h-${entry.label}`} className="plan-heading">
              {entry.label}
            </div>
          ) : (
            <div
              key={entry.item.id}
              className="item-row"
              onClick={() => openItemInPetaJournal(entry.item)}
            >
              <div className="item-main">
                <span className="item-page">{entry.item.name}</span>
                <span className="item-content">
                  <FmRoot ast={entry.item.ast} options={fmOptions} />
                </span>
                <ItemDates item={entry.item} />
              </div>
              <button
                className={`badge badge-${taskLabel(entry.item).toLowerCase()}`}
                onClick={e => {
                  e.stopPropagation();
                  cycleItemTask(entry.item);
                }}
                title="Click to cycle task"
              >
                {taskLabel(entry.item)}
              </button>
            </div>
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
