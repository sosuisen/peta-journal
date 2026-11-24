import { createContext, useContext, useSyncExternalStore } from 'react';
import type { Panel, Row } from './border/peta-types';
import type { Core } from './core';
import type { Panels } from './screen';

export const CoreContext = createContext<Core | null>(null);

export function useCore(): Core {
  const core = useContext(CoreContext);
  if (core === null) {
    throw new Error('useCore is called outside the CoreContext');
  }
  return core;
}

/**
 * The rows of the outline as the core last told. A component draws again only when the rows
 * change, not when a panel does.
 */
export function useRows(): readonly Row[] {
  const core = useCore();
  return useSyncExternalStore(core.subscribe, () => core.current().rows);
}

/**
 * A panel of the screen, or undefined before the core first sends it. A component draws again
 * only when this panel changes: a panel that does not come keeps its value.
 */
export function usePanel<K extends Panel['kind']>(kind: K): Panels<Panel>[K] {
  const core = useCore();
  return useSyncExternalStore(
    core.subscribe,
    () => core.current().panels[kind],
  );
}
