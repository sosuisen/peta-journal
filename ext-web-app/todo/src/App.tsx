import { useCallback, useEffect, useRef, useState } from 'react';
import { connect, getBlocksByStatus, updateBlockStatus, type Block } from './api';

type SortKey = 'file_name' | 'created_date' | 'status';

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

function blockCreatedMs(block: Block): number {
  if (block.createdDate && block.createdDate.millis) {
    const v = block.createdDate.millis;
    return typeof v === 'number' ? v : Number(v);
  }
  return 0;
}

function formatDate(block: Block): string {
  const ms = blockCreatedMs(block);
  if (!ms) return '';
  return new Date(ms).toLocaleString();
}

const STATUS_ORDER: Record<string, number> = { todo: 0, running: 1, done: 2 };
const NEXT_STATUS: Record<string, string> = { todo: 'running', running: 'done', done: 'todo' };

function sortBlocks(blocks: Block[], key: SortKey): Block[] {
  const sorted = [...blocks];
  sorted.sort((a, b) => {
    switch (key) {
      case 'file_name':
        return (a.fileName || '').localeCompare(b.fileName || '');
      case 'created_date':
        return blockCreatedMs(b) - blockCreatedMs(a); // desc
      case 'status':
        return (STATUS_ORDER[a.status || ''] ?? 9) - (STATUS_ORDER[b.status || ''] ?? 9);
    }
  });
  return sorted;
}

export default function App() {
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState('');
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [loading, setLoading] = useState(false);

  // Filters
  const [fromDate, setFromDate] = useState(todayStr());
  const [showTodo, setShowTodo] = useState(true);
  const [showRunning, setShowRunning] = useState(true);
  const [showDone, setShowDone] = useState(false);
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
        ? result.filter((b) => blockCreatedMs(b) <= new Date(fromDate).getTime() + 86400000 - 1)
        : result;
      setBlocks(filtered);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showTodo, showRunning, showDone, fromDate]);

  useEffect(() => {
    connect()
      .then(() => {
        connectedRef.current = true;
        setConnected(true);
        setError('');
        fetchBlocks();
      })
      .catch((e) => setError('Connection failed: ' + e));
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (connected) fetchBlocks();
  }, [showTodo, showRunning, showDone, fromDate]);  // eslint-disable-line react-hooks/exhaustive-deps

  const openPage = (block: Block) => {
    const pageId = block.pageId;
    if (pageId) {
      window.open(`http://localhost:4200/pages/${pageId}`, 'peta-journal');
    }
  };

  const handleStatusToggle = async (block: Block) => {
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

  const sorted = sortBlocks(blocks, sortKey);

  return (
    <div className="app">
      <h1>PetaJournal Todo</h1>

      {error && <div className="error">{error}</div>}
      {!connected && !error && <div className="connecting">Connecting...</div>}

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

        <label>
          Sort:
          <select value={sortKey} onChange={(e) => setSortKey(e.target.value as SortKey)}>
            <option value="created_date">Created Date</option>
            <option value="file_name">Page Name</option>
            <option value="status">Status</option>
          </select>
        </label>

        <button onClick={fetchBlocks} disabled={loading}>
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      <div className="block-list">
        {sorted.length === 0 && !loading && <div className="empty">No blocks found</div>}
        {sorted.map((block) => (
          <div key={block.id} className="block-row" onClick={() => openPage(block)}>
            <span className="block-page">{block.fileName}</span>
            <span
              className="block-content"
              dangerouslySetInnerHTML={{ __html: block.html || block.body || '' }}
            />
            <span className="block-date">{formatDate(block)}</span>
            <button
              className={`badge badge-${block.status}`}
              onClick={(e) => { e.stopPropagation(); handleStatusToggle(block); }}
              title="Click to cycle status"
            >
              {block.status}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
