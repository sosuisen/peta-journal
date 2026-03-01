import { useCallback, useEffect, useRef, useState } from 'react';
import { connect, onConnectionStateChange, onServerEvent, getItemsByStatus, updateItemStatus, type ConnectionState, type Item } from './api';

type SortKey = 'name' | 'created_date' | 'status';

/** Returns today's date as an ISO string (YYYY-MM-DD). */
function getTodayISODate(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Formats an item's creation date for display using the browser's locale. */
function formatDate(item: Item): string {
  if (!item.createdDate) return '';
  return new Date(item.createdDate).toLocaleDateString();
}

/** Checks if an item belongs to a journal page (filename matches YYYY-MM-DD). */
function isJournal(item: Item): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(item.name || '');
}

const STATUS_ORDER: Record<string, number> = { todo: 0, running: 1, done: 2 };
const NEXT_STATUS: Record<string, string> = { todo: 'running', running: 'done', done: 'todo' };

/** Sorts items by the specified key, returning a new array. */
function sortItems(items: Item[], key: SortKey): Item[] {
  const sorted = [...items];
  sorted.sort((a, b) => {
    switch (key) {
      case 'name':
        return (a.name || '').localeCompare(b.name || '');
      case 'created_date':
        return (b.createdDate || 0) - (a.createdDate || 0); // desc
      case 'status':
        return (STATUS_ORDER[a.status || ''] ?? 9) - (STATUS_ORDER[b.status || ''] ?? 9);
    }
  });
  return sorted;
}

/** Main todo application component. Displays items with status filtering, sorting, and cycling. */
export default function App() {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [error, setError] = useState('');
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);

  // Filters
  const [fromDate, setFromDate] = useState(getTodayISODate());
  const [showTodo, setShowTodo] = useState(true);
  const [showRunning, setShowRunning] = useState(true);
  const [showDone, setShowDone] = useState(false);
  const [showJournal, setShowJournal] = useState(true);
  const [showPage, setShowPage] = useState(false);
  const [sortKey, setSortKey] = useState<SortKey>('created_date');

  const connectedRef = useRef(false);

  const fetchItems = useCallback(async () => {
    if (!connectedRef.current) return;
    const statuses: string[] = [];
    if (showTodo) statuses.push('todo');
    if (showRunning) statuses.push('running');
    if (showDone) statuses.push('done');
    if (statuses.length === 0) {
      setItems([]);
      return;
    }
    setLoading(true);
    try {
      const result = await getItemsByStatus(statuses);
      // Filter by date client-side: show items created on or before the selected date
      const filtered = fromDate
        ? result.filter((b) => (b.createdDate || 0) <= new Date(fromDate).getTime() + 86400000 - 1)
        : result;
      setItems(filtered);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showTodo, showRunning, showDone, fromDate]);

  const retryTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const scheduleRetry = useCallback(() => {
    if (retryTimer.current) return; // already scheduled
    retryTimer.current = setTimeout(() => {
      retryTimer.current = null;
      connectWithRetry();
    }, 3000);
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

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
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onConnectionStateChange((state) => {
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
  }, [connectWithRetry]);  // eslint-disable-line react-hooks/exhaustive-deps

  const connected = connectionState === 'connected';

  useEffect(() => {
    if (connected) fetchItems();
  }, [showTodo, showRunning, showDone, fromDate]);  // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onServerEvent(() => {
      fetchItems();
    });
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  const openItemInPetaJournal = (item: Item) => {
    const name = item.name;
    if (!name) return;
    const isJournal = /^\d{4}(-\d{2}(-\d{2})?|-W\d{1,2})?$/.test(name);
    const prefix = isJournal ? '/journals' : '/docs';
    const path = `${prefix}/${item.dirPath || ''}${name}`;
    window.open(`http://localhost:4200${path}#${item.id}`, 'peta-journal');
  };

  const cycleItemStatus = async (item: Item) => {
    const next = NEXT_STATUS[item.status || 'todo'] || 'todo';
    try {
      const updated = await updateItemStatus(item.id!, next);
      setItems((prev) =>
        prev.map((b) => (b.id === updated.id ? updated : b))
          .filter((b) => {
            const statuses: string[] = [];
            if (showTodo) statuses.push('todo');
            if (showRunning) statuses.push('running');
            if (showDone) statuses.push('done');
            return statuses.includes(b.status || '');
          })
      );
    } catch (e) {
      setError(String(e));
    }
  };

  const filtered = items.filter((b) => {
    if (isJournal(b)) return showJournal;
    return showPage;
  });
  const sorted = sortItems(filtered, sortKey);

  return (
    <div className="app">
      <h1>PetaApps ToDo</h1>

      {error && <div className="error">{error}</div>}

      <div className="filter-bar">
        <label>
          Until:
          <input type="date" value={fromDate} onChange={(e) => setFromDate(e.target.value)} />
        </label>

        <label className="checkbox">
          <input type="checkbox" checked={showTodo} onChange={(e) => setShowTodo(e.target.checked)} />
          Todo
        </label>
        <label className="checkbox">
          <input type="checkbox" checked={showRunning} onChange={(e) => setShowRunning(e.target.checked)} />
          Running
        </label>
        <label className="checkbox">
          <input type="checkbox" checked={showDone} onChange={(e) => setShowDone(e.target.checked)} />
          Done
        </label>

        <label className="checkbox">
          <input type="checkbox" checked={showJournal} onChange={(e) => setShowJournal(e.target.checked)} />
          Journal
        </label>
        <label className="checkbox">
          <input type="checkbox" checked={showPage} onChange={(e) => setShowPage(e.target.checked)} />
          Page
        </label>

        <label>
          Sort:
          <select value={sortKey} onChange={(e) => setSortKey(e.target.value as SortKey)}>
            <option value="created_date">Created Date</option>
            <option value="name">Page Name</option>
            <option value="status">Status</option>
          </select>
        </label>

      </div>

      <div className="item-list">
        {sorted.length === 0 && !loading && <div className="empty">No items found</div>}
        {sorted.map((item) => (
          <div key={item.id} className="item-row" onClick={() => openItemInPetaJournal(item)}>
            <div className="item-main">
              <span className="item-page">{item.name}</span>
              <span
                className="item-content"
                dangerouslySetInnerHTML={{ __html: item.html || item.body || '' }}
              />
            </div>
            <span className="item-date">{formatDate(item)}</span>
            <button
              className={`badge badge-${item.status}`}
              onClick={(e) => { e.stopPropagation(); cycleItemStatus(item); }}
              title="Click to cycle status"
            >
              {item.status}
            </button>
          </div>
        ))}
      </div>
      <footer className="footer">
        <span className={`connection-status ${connectionState}`}>{connectionState}</span>
      </footer>
    </div>
  );
}
