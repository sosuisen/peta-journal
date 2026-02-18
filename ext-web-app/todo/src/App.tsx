import { useCallback, useEffect, useRef, useState } from 'react';
import { connect, onConnectionStateChange, onServerEvent, getBlocksByStatus, updateBlockStatus, type ConnectionState, type Block } from './api';

type SortKey = 'file_name' | 'created_date' | 'status';

/** Returns today's date as an ISO string (YYYY-MM-DD). */
function getTodayISODate(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Extracts the creation timestamp in milliseconds from a block. Returns 0 if unavailable. */
function getBlockCreatedMillis(block: Block): number {
  if (block.createdDate && block.createdDate.millis) {
    const v = block.createdDate.millis;
    return typeof v === 'number' ? v : Number(v);
  }
  return 0;
}

/** Formats a block's creation date for display using the browser's locale. */
function formatDate(block: Block): string {
  const ms = getBlockCreatedMillis(block);
  if (!ms) return '';
  return new Date(ms).toLocaleDateString();
}

/** Checks if a block belongs to a journal page (filename matches YYYY-MM-DD). */
function isJournal(block: Block): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(block.fileName || '');
}

const STATUS_ORDER: Record<string, number> = { todo: 0, running: 1, done: 2 };
const NEXT_STATUS: Record<string, string> = { todo: 'running', running: 'done', done: 'todo' };

/** Sorts blocks by the specified key, returning a new array. */
function sortBlocks(blocks: Block[], key: SortKey): Block[] {
  const sorted = [...blocks];
  sorted.sort((a, b) => {
    switch (key) {
      case 'file_name':
        return (a.fileName || '').localeCompare(b.fileName || '');
      case 'created_date':
        return getBlockCreatedMillis(b) - getBlockCreatedMillis(a); // desc
      case 'status':
        return (STATUS_ORDER[a.status || ''] ?? 9) - (STATUS_ORDER[b.status || ''] ?? 9);
    }
  });
  return sorted;
}

/** Main todo application component. Displays blocks with status filtering, sorting, and cycling. */
export default function App() {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
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

  const fetchBlocks = useCallback(async () => {
    if (!connectedRef.current) return;
    const statuses: string[] = [];
    if (showTodo) statuses.push('todo');
    if (showRunning) statuses.push('running');
    if (showDone) statuses.push('done');
    if (statuses.length === 0) {
      setBlocks([]);
      return;
    }
    setLoading(true);
    try {
      const result = await getBlocksByStatus(statuses);
      // Filter by date client-side: show blocks created on or before the selected date
      const filtered = fromDate
        ? result.filter((b) => getBlockCreatedMillis(b) <= new Date(fromDate).getTime() + 86400000 - 1)
        : result;
      setBlocks(filtered);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showTodo, showRunning, showDone, fromDate]);

  const connectWithRetry = useCallback(() => {
    setError('');
    connect()
      .then(() => {
        connectedRef.current = true;
        setError('');
        fetchBlocks();
      })
      .catch(() => {
        connectedRef.current = false;
        setTimeout(connectWithRetry, 3000);
      });
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onConnectionStateChange((state) => {
      setConnectionState(state);
      if (state === 'disconnected' || state === 'error') {
        connectedRef.current = false;
        setTimeout(connectWithRetry, 3000);
      }
    });
    connectWithRetry();
  }, [connectWithRetry]);

  const connected = connectionState === 'connected';

  useEffect(() => {
    if (connected) fetchBlocks();
  }, [showTodo, showRunning, showDone, fromDate]);  // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    onServerEvent((event) => {
      if (event.pagesUpdated) {
        fetchBlocks();
      }
    });
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  const openBlockInPetaJournal = (block: Block) => {
    const fileName = block.fileName;
    if (!fileName) return;
    const isJournal = /^\d{4}(-\d{2}(-\d{2})?|-W\d{1,2})?$/.test(fileName);
    const prefix = isJournal ? '/journals' : '/docs';
    const path = `${prefix}/${block.dirPath || ''}${fileName}`;
    window.open(`http://localhost:4200${path}#${block.id}`, 'peta-journal');
  };

  const cycleBlockStatus = async (block: Block) => {
    const next = NEXT_STATUS[block.status || 'todo'] || 'todo';
    try {
      const updated = await updateBlockStatus(block.id!, next);
      setBlocks((prev) =>
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

  const filtered = blocks.filter((b) => {
    if (isJournal(b)) return showJournal;
    return showPage;
  });
  const sorted = sortBlocks(filtered, sortKey);

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
            <option value="file_name">Page Name</option>
            <option value="status">Status</option>
          </select>
        </label>

      </div>

      <div className="block-list">
        {sorted.length === 0 && !loading && <div className="empty">No blocks found</div>}
        {sorted.map((block) => (
          <div key={block.id} className="block-row" onClick={() => openBlockInPetaJournal(block)}>
            <div className="block-main">
              <span className="block-page">{block.fileName}</span>
              <span
                className="block-content"
                dangerouslySetInnerHTML={{ __html: block.html || block.body || '' }}
              />
            </div>
            <span className="block-date">{formatDate(block)}</span>
            <button
              className={`badge badge-${block.status}`}
              onClick={(e) => { e.stopPropagation(); cycleBlockStatus(block); }}
              title="Click to cycle status"
            >
              {block.status}
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
