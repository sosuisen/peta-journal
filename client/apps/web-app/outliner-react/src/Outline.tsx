import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useRef, type ReactNode } from 'react';
import { useAsked } from './askedContext';
import { useCore, usePanel, useRows } from './context';
import type { Core } from './core';
import { chordOf } from './keys';
import { OutlineItem } from './OutlineItem';
import WikiSuggestions from './WikiSuggestions';

/** The height of a row that was never measured. */
const ESTIMATED_ROW_PX = 28;
/** The share of the height of the list left empty below the last row. */
const BOTTOM_ROOM_RATIO = 0.5;

interface Props {
  /** Drawn above the rows, and scrolled with them. */
  readonly header?: ReactNode;
  /** Drawn below the rows, and scrolled with them. */
  readonly footer?: ReactNode;
}

/**
 * The outline: a virtual list of the rows (ADR 080). Only the rows near the screen are in the
 * page; padding stands in for the rest.
 */
export default function Outline({ header, footer }: Props) {
  const core = useCore();
  const asked = useAsked();
  const rows = useRows();
  const outline = usePanel('Outline');
  const viewport = useRef<HTMLElement>(null);
  const headerBox = useRef<HTMLDivElement>(null);
  // The app does not use the React Compiler, which this warns about: the outline is drawn
  // again whenever the rows change, and the rows memo themselves
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => viewport.current,
    estimateSize: () => ESTIMATED_ROW_PX,
    getItemKey: index => rows[index].item.id,
    overscan: 10,
    scrollMargin: headerBox.current?.offsetHeight ?? 0,
  });

  // A scroll to an item puts it a tenth down the list, as the sidebar and a search ask
  useEffect(() => {
    asked.scroller = (itemId, smooth) => {
      const index = core
        .current()
        .rows.findIndex(row => row.item.id === itemId);
      const list = viewport.current;
      if (index < 0 || list === null) {
        return;
      }
      const [offset] = virtualizer.getOffsetForIndex(index, 'start') ?? [0];
      list.scrollTo({
        top: Math.max(0, offset - list.clientHeight * 0.1),
        behavior: smooth ? 'smooth' : 'auto',
      });
    };
    return () => {
      asked.scroller = null;
    };
  }, [asked, core, virtualizer]);

  useSelectionKeys(core, outline?.selection_mode ?? false);

  const items = virtualizer.getVirtualItems();
  const margin = virtualizer.options.scrollMargin;
  const first = items[0];
  const last = items[items.length - 1];
  const padTop = first === undefined ? 0 : first.start - margin;
  const room = (viewport.current?.clientHeight ?? 0) * BOTTOM_ROOM_RATIO;
  const padBottom =
    (last === undefined
      ? 0
      : virtualizer.getTotalSize() - (last.end - margin)) + room;

  const scrolled = (list: HTMLElement) => {
    const top = list.scrollTop;
    const shown = virtualizer.getVirtualItems();
    const topVisible = shown.find(item => item.end > top)?.index ?? 0;
    core.send({
      kind: 'OutlineScrolled',
      scrolled_from_top: top > 0,
      top_visible_index: BigInt(topVisible),
      rendered_start: BigInt(shown[0]?.index ?? 0),
      rendered_end: BigInt((shown[shown.length - 1]?.index ?? -1) + 1),
    });
  };

  const request = outline?.focus_request ?? null;
  const holdingKeys = outline?.key_relay_holding ?? false;

  return (
    <app-outline>
      <cdk-virtual-scroll-viewport
        ref={viewport}
        className="outline-viewport"
        onScroll={event => scrolled(event.currentTarget)}
      >
        {header !== undefined && (
          <div ref={headerBox} className="outline-header">
            {header}
          </div>
        )}
        <div
          className="cdk-virtual-scroll-content-wrapper"
          style={{ paddingTop: `${padTop}px`, paddingBottom: `${padBottom}px` }}
        >
          {items.map(item => {
            const row = rows[item.index];
            return (
              <div
                key={item.key}
                className="outline-row"
                data-row-id={row.item.id}
                data-index={item.index}
                ref={virtualizer.measureElement}
                onWheel={event => {
                  if (event.shiftKey) {
                    core.send({
                      kind: 'ZoomStep',
                      item_id: row.item.id,
                      up: event.deltaY < 0,
                    });
                  }
                }}
              >
                {row.date_header !== null && (
                  <div className="date-header">{row.date_header}</div>
                )}
                <OutlineItem
                  row={row}
                  request={row.is_editing ? request : null}
                  holdingKeys={row.is_editing && holdingKeys}
                />
              </div>
            );
          })}
        </div>
        {footer !== undefined && <div className="outline-footer">{footer}</div>}
      </cdk-virtual-scroll-viewport>
      <WikiSuggestions />
    </app-outline>
  );
}

/** Whether an event comes from the confirm dialog, which answers its own keys and clicks. */
function inDialog(event: Event): boolean {
  return (
    event.target instanceof Element &&
    event.target.closest('.confirm-overlay') !== null
  );
}

/**
 * The keys and the clicks of the whole page while items are selected. Undo and Redo also
 * reach the page with no editor and no selection, as right after the selection was deleted.
 */
function useSelectionKeys(core: Core, selecting: boolean): void {
  useEffect(() => {
    const pressed = (event: KeyboardEvent) => {
      if (inDialog(event)) {
        return;
      }
      const onPage = event.target === document.body;
      if (!selecting && !onPage) {
        return;
      }
      const chord = chordOf(event);
      if (chord === null) {
        return;
      }
      const undoOrRedo =
        (event.ctrlKey || event.metaKey) &&
        ['z', 'y'].includes(event.key.toLowerCase());
      if (!selecting && !undoOrRedo) {
        return;
      }
      if (isCopy(event)) {
        core.beginCopy();
      }
      if (core.press({ kind: 'InSelection', chord })) {
        event.preventDefault();
      }
    };
    document.addEventListener('keydown', pressed);
    if (!selecting) {
      return () => document.removeEventListener('keydown', pressed);
    }
    const clicked = (event: MouseEvent) => {
      if (!inDialog(event)) {
        core.send({ kind: 'SelectionClickOutside' });
      }
    };
    const copied = (event: ClipboardEvent) => {
      event.preventDefault();
      core.beginCopy();
      const key = event.type === 'cut' ? 'x' : 'c';
      core.press({
        kind: 'InSelection',
        chord: {
          key: { kind: 'Letter', value: key },
          shift: false,
          ctrl: true,
          alt: false,
          cmd: false,
        },
      });
    };
    document.addEventListener('click', clicked);
    document.addEventListener('copy', copied);
    document.addEventListener('cut', copied);
    return () => {
      document.removeEventListener('keydown', pressed);
      document.removeEventListener('click', clicked);
      document.removeEventListener('copy', copied);
      document.removeEventListener('cut', copied);
    };
  }, [core, selecting]);
}

/** Ctrl or Command with C or X, which write the clipboard. */
function isCopy(event: KeyboardEvent): boolean {
  return (
    (event.ctrlKey || event.metaKey) &&
    ['c', 'x'].includes(event.key.toLowerCase())
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-outline': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'cdk-virtual-scroll-viewport': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
