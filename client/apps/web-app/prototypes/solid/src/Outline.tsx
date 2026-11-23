import {
  elementScroll,
  observeElementOffset,
  observeElementRect,
  Virtualizer,
  type VirtualItem,
} from '@tanstack/virtual-core';
import { createEffect, createMemo, createSignal, For, onCleanup, onSettled, Show } from 'solid-js';
import type { Core } from '../../shared/src/core';
import { placeCursor, pressInEditor, typedInEditor } from '../../shared/src/keys';
import { measure } from '../../shared/src/metrics';
import type { FocusRequest, OutlinePanel, Row } from '../../shared/src/types';

export interface OutlineHandle {
  focus(itemId: string | null): void;
  scrollTo(itemId: string): void;
}

interface Props {
  readonly core: Core;
  readonly rows: readonly Row[];
  readonly outline: OutlinePanel | null;
  readonly handle: (handle: OutlineHandle) => void;
}

/**
 * The outline, with the virtual list of `@tanstack/virtual-core`. Its Solid adapter
 * (`@tanstack/solid-virtual`) asks for Solid 1, so the core is wired here by hand.
 */
export default function Outline(props: Props) {
  let scroller: HTMLDivElement | undefined;
  const [tick, setTick] = createSignal(0);
  const virtualizer = new Virtualizer<HTMLDivElement, HTMLDivElement>({
    count: 0,
    getScrollElement: () => scroller ?? null,
    estimateSize: () => 26,
    overscan: 8,
    scrollToFn: elementScroll,
    observeElementRect,
    observeElementOffset,
    onChange: () => setTick(t => t + 1),
  });

  // The list follows the number and the keys of the rows
  createEffect(
    () => props.rows.map(row => row.item.id),
    ids => {
      virtualizer.setOptions({
        ...virtualizer.options,
        count: ids.length,
        getItemKey: index => ids[index] ?? index,
      });
      virtualizer._willUpdate();
      setTick(t => t + 1);
    },
  );
  onSettled(() => {
    const unmount = virtualizer._didMount();
    virtualizer._willUpdate();
    return unmount;
  });

  const items = createMemo((): VirtualItem[] => {
    tick();
    return virtualizer.getVirtualItems();
  });
  const total = createMemo(() => {
    tick();
    return virtualizer.getTotalSize();
  });

  const scrollTo = (itemId: string | null) => {
    const index = props.rows.findIndex(row =>
      itemId === null ? row.is_editing : row.item.id === itemId,
    );
    if (index >= 0) {
      virtualizer.scrollToIndex(index, { align: 'auto' });
    }
  };

  // The editor takes the focus itself once it is drawn. The outline only brings it on screen
  props.handle({ focus: scrollTo, scrollTo });
  const request = () => props.outline?.focus_request ?? null;

  return (
    <div ref={element => (scroller = element)} class="outline">
      <div style={{ height: `${total()}px`, position: 'relative' }}>
        <For each={items()} keyed={virtual => virtual.key}>
          {virtual => (
            <Show when={props.rows[virtual().index]}>
              {row => (
                <div
                  data-index={virtual().index}
                  ref={element => queueMicrotask(() => virtualizer.measureElement(element))}
                  style={{
                    position: 'absolute',
                    top: '0',
                    left: '0',
                    right: '0',
                    transform: `translateY(${virtual().start}px)`,
                  }}
                >
                  <ItemRow core={props.core} row={row()} request={request()} />
                </div>
              )}
            </Show>
          )}
        </For>
      </div>
    </div>
  );
}

/** One row. Each field is drawn by itself, so a change redraws only what it touches. */
function ItemRow(props: { core: Core; row: Row; request: FocusRequest | null }) {
  const id = () => props.row.item.id;
  return (
    <>
      <Show when={props.row.date_header}>
        {header => <div class="date-header">{header()}</div>}
      </Show>
      <div
        class={props.row.is_selected ? 'row selected' : 'row'}
        style={{ 'padding-left': `${props.row.indent_px}px` }}
      >
        <span class="bullet" onClick={() => props.core.send({ CollapseToggle: { item_id: id() } })}>
          {props.row.has_children ? (props.row.is_collapsed ? '▸' : '▾') : '•'}
        </span>
        <div class="body">
          <Show
            when={props.row.is_editing}
            fallback={
              <div
                // On the press, as the web shell does: the editor takes the focus in this very
                // handler, and the focus change of the browser would take it away again
                onMouseDown={event => {
                  event.preventDefault();
                  props.core.send({ EditRequested: { item_id: id(), place: 'Unplaced' } });
                }}
                innerHTML={props.row.body_model}
              />
            }
          >
            <Editor
              core={props.core}
              itemId={id()}
              body={props.row.item.body}
              request={props.request}
            />
          </Show>
        </div>
      </div>
    </>
  );
}

/**
 * The editor of an item. Its text is the browser's while the user types: the core's body is
 * put in only when the two differ, so that the IME keeps what it is composing.
 */
function Editor(props: { core: Core; itemId: string; body: string; request: FocusRequest | null }) {
  let area: HTMLTextAreaElement | undefined;
  /** The number of the last request this editor followed. */
  let followed: number | null = null;
  createEffect(
    () => props.body,
    body => {
      if (area !== undefined && area.value !== body) {
        area.value = body;
      }
    },
  );
  // The focus, on mount and whenever the core asks again
  createEffect(
    () => props.request,
    request => {
      if (area === undefined) {
        return;
      }
      const mine = request !== null && request.item_id === props.itemId ? request : null;
      if (mine !== null && mine.seq !== followed) {
        followed = mine.seq;
        placeCursor(area, mine.cursor);
      } else if (followed === null) {
        followed = -1;
        placeCursor(area, null);
      }
    },
  );
  onCleanup(() => {
    area = undefined;
  });
  return (
    <textarea
      ref={element => {
        area = element;
        element.value = props.body;
      }}
      data-id={props.itemId}
      rows={1}
      onKeyDown={event => {
        measure('key');
        pressInEditor(props.core, props.itemId, event.currentTarget, event);
      }}
      onInput={event => typedInEditor(props.core, props.itemId, event.currentTarget)}
      onBlur={() => props.core.send({ EditFinished: { item_id: props.itemId } })}
    />
  );
}
