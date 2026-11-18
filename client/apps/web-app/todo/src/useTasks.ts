import { useEffect, useState } from 'react';
import {
  cycleTask,
  getLinkConfig,
  getTasks,
  subscribe,
  type ConnectionState,
  type Item,
  type LinkConfig,
} from './api';

/**
 * The tasks of the journal server. They are loaded when the app connects, and again
 * whenever a page changes on the server.
 *
 * Without `includeDone` only the tasks that are not done are loaded. Most tasks are done,
 * so this keeps the app fast to open. The done tasks are loaded when they are asked for.
 *
 * The folder of the tag pages comes with them, because a hashtag in the text of a task
 * links into it. It is loaded when the app connects.
 */
export function useTasks(includeDone: boolean) {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('disconnected');
  const [error, setError] = useState('');
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);
  const [links, setLinks] = useState<LinkConfig>();
  // Counts the changes of pages on the server; a new count loads the tasks again
  const [pageChanges, setPageChanges] = useState(0);

  useEffect(
    () =>
      subscribe({
        onStateChange: state => {
          setConnectionState(state);
          // Each try to connect takes away the error of a request that failed before it
          if (state !== 'disconnected') setError('');
        },
        onPagesUpdated: () => setPageChanges(count => count + 1),
      }),
    [],
  );

  const connected = connectionState === 'connected';

  useEffect(() => {
    if (!connected) return;
    // A later load starts before this one answers when the filter changes quickly
    let outdated = false;
    const loadItems = async () => {
      setLoading(true);
      try {
        const tasks = await getTasks(includeDone);
        if (!outdated) setItems(tasks);
      } catch (e) {
        if (!outdated) setError(String(e));
      } finally {
        if (!outdated) setLoading(false);
      }
    };
    void loadItems();
    return () => {
      outdated = true;
    };
  }, [connected, includeDone, pageChanges]);

  useEffect(() => {
    if (!connected) return;
    const loadLinks = async () => {
      try {
        setLinks(await getLinkConfig());
      } catch (e) {
        setError(String(e));
      }
    };
    void loadLinks();
  }, [connected]);

  /** Moves the task to its next state: TODO, RUNNING, DONE and back to TODO. */
  const cycle = async (item: Item) => {
    try {
      const updated = await cycleTask(item.id);
      setItems(prev => prev.map(b => (b.id === updated.id ? updated : b)));
    } catch (e) {
      setError(String(e));
    }
  };

  return { items, loading, error, connectionState, links, cycle };
}
