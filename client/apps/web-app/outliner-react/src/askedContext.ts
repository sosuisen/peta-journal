import { createContext } from 'react';
import type { AskedOps } from './asked';

/** The operations of the screen that the core asks the shell to do. */
export const AskedOpsContext = createContext<AskedOps | null>(null);
