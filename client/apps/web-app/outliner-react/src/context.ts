import { createContext, useContext, useSyncExternalStore } from 'react';
import type { Panel } from './border/peta-types';
import type { Core, Screen } from './core';
import type { Panels } from './screen';

export const CoreContext = createContext<Core | null>(null);

export function useCore(): Core {
  const core = useContext(CoreContext);
  if (core === null) {
    throw new Error('useCore is called outside the CoreContext');
  }
  return core;
}

/** The screen as the core last told. A component draws again when it changes. */
export function useScreen(): Screen {
  const core = useCore();
  return useSyncExternalStore(core.subscribe, core.current);
}

/** A panel of the screen, or undefined before the core first sends it. */
export function usePanel<K extends Panel['kind']>(kind: K): Panels<Panel>[K] {
  return useScreen().panels[kind];
}
