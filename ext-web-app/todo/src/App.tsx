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
import {
  getDue,
  getPlan,
  groupByPlan,
  isShown,
  pagePath,
  taskLabel,
} from './tasks';

/** Returns today's date as an ISO string (YYYY-MM-DD). */
function getTodayISODate(): string {
  return new Date().toISOString().slice(0, 10);
}

function ItemDates({ item }: { item: Item }) {
  const plan = getPlan(item);
  const due = getDue(item);
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
    window.open(`${buildBaseUrl()}${pagePath(item)}`, 'peta-journal');
  };

  const cycleItemTask = async (item: Item) => {
    try {
      const updated = await cycleTask(item.id!);
      setItems(prev => prev.map(b => (b.id === updated.id ? updated : b)));
    } catch (e) {
      setError(String(e));
    }
  };

  const filters = { untilDate, showDone, showJournal, showPage };
  const filtered = items.filter(item => isShown(item, filters));

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
