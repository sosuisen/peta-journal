import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

// The real wasm talks to the server; the tests use a shell that keeps what it is handed
vi.mock('../wasm/bridge', () => import('./shell'));

// Testing Library removes the drawn components by itself only with the globals of Vitest
afterEach(cleanup);
