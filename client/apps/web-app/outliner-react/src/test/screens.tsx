/** Rows, panels and a drawn shell for the tests of the components. */
import { render, type RenderResult } from '@testing-library/react';
import type { ReactNode } from 'react';
import { AskedOps } from '../asked';
import { AskedOpsContext } from '../askedContext';
import {
  FocusRequest,
  Item,
  OutlinePanel,
  Row,
  WikiView,
  type PropertyValue,
} from '../border/peta-types';
import { CoreContext } from '../context';
import { Core } from '../core';
import { WebShell } from './shell';

/** An item that is shown and not edited, with the body `body`. Each test changes what it needs. */
export function row(
  id: string,
  body = '',
  changes: Partial<Row> = {},
  properties: Record<string, string> = {},
): Row {
  const props = new Map<string, PropertyValue>(
    Object.entries(properties).map(([key, value]) => [
      key,
      { kind: 'Text', value },
    ]),
  );
  const item = new Item(
    id,
    'page',
    '',
    'notes',
    'doc',
    0n,
    0n,
    'a',
    1,
    body,
    false,
    [],
    props,
    null,
  );
  const shown = new Row(
    item,
    `<p>${body}</p>`,
    0n,
    0,
    false,
    false,
    false,
    false,
    0,
    false,
    false,
    1,
    null,
    false,
    false,
    false,
    true,
    1,
    null,
    false,
  );
  return Object.assign(shown, changes);
}

/** The panel of the outline with nothing selected, no popup and no order of the core. */
export function outline(changes: Partial<OutlinePanel> = {}): OutlinePanel {
  const panel = new OutlinePanel(
    false,
    null,
    null,
    null,
    false,
    new WikiView(null, [], 0n, 0, 0),
    false,
  );
  return Object.assign(panel, changes);
}

/** The order to put the cursor of the item `itemId` at the place `cursor`. */
export function focusAt(seq: number, itemId: string, cursor: number | null) {
  return new FocusRequest(
    BigInt(seq),
    itemId,
    cursor === null ? null : BigInt(cursor),
  );
}

/** The core, its wasm of the tests, and the operations it asks the shell for. */
export interface Shell {
  readonly core: Core;
  readonly wasm: WebShell;
  readonly asked: AskedOps;
}

/** A core on the wasm of the tests, wired as `main.tsx` wires the real one. */
export function shell(): Shell {
  const asked = new AskedOps();
  const core = new Core(asked.take);
  asked.answerTo(core);
  return { core, wasm: WebShell.last, asked };
}

/** Draws `ui` inside the contexts of the shell. `rerender` keeps them. */
export function draw({ core, asked }: Shell, ui: ReactNode): RenderResult {
  return render(ui, {
    wrapper: ({ children }) => (
      <CoreContext value={core}>
        <AskedOpsContext value={asked}>{children}</AskedOpsContext>
      </CoreContext>
    ),
  });
}
