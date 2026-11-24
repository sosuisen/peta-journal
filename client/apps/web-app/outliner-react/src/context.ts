import { createContext, use, useContext, useSyncExternalStore } from 'react';
import type { Panel, Row } from './border/peta-types';
import type { Core } from './core';
import type { Panels } from './screen';

/**
 * The core, or the promise of it while the wasm loads. `main.tsx` gives the promise, so the
 * frame is on the page before the wasm is.
 */
export const CoreContext = createContext<Core | Promise<Core> | null>(null);

/**
 * The core. While the wasm loads, the component waits for it in the nearest `Suspense`, and a
 * wasm that fails to load is thrown to the nearest error boundary.
 */
export function useCore(): Core {
  const core = useContext(CoreContext);
  if (core === null) {
    throw new Error('useCore is called outside the CoreContext');
  }
  return core instanceof Promise ? use(core) : core;
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
