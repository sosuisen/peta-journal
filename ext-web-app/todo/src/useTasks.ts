import { useEffect, useState } from 'react';
import {
  cycleTask,
  getItemsByTypes,
  getTagsDir,
  subscribe,
  type ConnectionState,
  type Item,
} from './api';

/**
 * The tasks of the journal server. They are loaded when the app connects, and again
 * whenever a page changes on the server.
 *
 * The folder of the tag pages comes with them, because a hashtag in the text of a task
 * links into it. It is loaded when the app connects.
 */
export function useTasks() {
  const [connectionState, setConnectionState] =
    useState<ConnectionState>('disconnected');
  const [error, setError] = useState('');
  const [items, setItems] = useState<Item[]>([]);
  const [loading, setLoading] = useState(false);
  const [tagsDir, setTagsDir] = useState('');

  useEffect(() => {
    const loadItems = async () => {
      setLoading(true);
      try {
        setItems(await getItemsByTypes(['task']));
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    };
    const loadTagsDir = async () => {
      try {
        setTagsDir(await getTagsDir());
      } catch (e) {
        setError(String(e));
      }
    };
    return subscribe({
      onStateChange: state => {
        setConnectionState(state);
        // Each try to connect takes away the error of a request that failed before it
        if (state !== 'disconnected') setError('');
        if (state === 'connected') {
          void loadItems();
          void loadTagsDir();
        }
      },
      onPagesUpdated: () => void loadItems(),
    });
  }, []);

  /** Moves the task to its next state: TODO, RUNNING, DONE and back to TODO. */
  const cycle = async (item: Item) => {
    try {
      const updated = await cycleTask(item.id);
      setItems(prev => prev.map(b => (b.id === updated.id ? updated : b)));
    } catch (e) {
      setError(String(e));
    }
  };

  return { items, loading, error, connectionState, tagsDir, cycle };
}
