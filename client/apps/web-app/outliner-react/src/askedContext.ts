import { createContext, useContext } from 'react';
import type { AskedOps } from './asked';

/** The operations of the screen that the core asks the shell to do. */
export const AskedOpsContext = createContext<AskedOps | null>(null);

export function useAsked(): AskedOps {
  const asked = useContext(AskedOpsContext);
  if (asked === null) {
    throw new Error('useAsked is called outside the AskedOpsContext');
  }
  return asked;
}
