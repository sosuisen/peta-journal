// The measurements of #971. Each one starts at an input and ends when the browser has
// painted the frame after it: the next animation frame, and then a task after that frame.

export type Kind = 'key' | 'open' | 'expand';

const samples: Record<Kind, number[]> = { key: [], open: [], expand: [] };

/** Starts a measurement of `kind`, which ends after the next paint. */
export function measure(kind: Kind): void {
  const start = performance.now();
  requestAnimationFrame(() => {
    setTimeout(() => samples[kind].push(performance.now() - start), 0);
  });
}

/** A measurement that ends when `done` says so, checked after each paint. */
export function measureUntil(kind: Kind, done: () => boolean): void {
  const start = performance.now();
  const check = () => {
    if (done()) {
      samples[kind].push(performance.now() - start);
    } else {
      requestAnimationFrame(() => setTimeout(check, 0));
    }
  };
  requestAnimationFrame(() => setTimeout(check, 0));
}

export interface Summary {
  readonly count: number;
  readonly median: number;
  readonly p95: number;
}

function summary(values: readonly number[]): Summary {
  const sorted = [...values].sort((a, b) => a - b);
  const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))] ?? 0;
  return { count: sorted.length, median: at(0.5), p95: at(0.95) };
}

export function report(): Record<Kind, Summary> {
  return {
    key: summary(samples.key),
    open: summary(samples.open),
    expand: summary(samples.expand),
  };
}

export function clear(): void {
  for (const kind of Object.keys(samples) as Kind[]) {
    samples[kind].length = 0;
  }
}
