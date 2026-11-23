import { useVirtualizer } from '@tanstack/react-virtual';
import { memo, useImperativeHandle, useLayoutEffect, useRef, type Ref } from 'react';
import type { Core } from '../../shared/src/core';
import { placeCursor, pressInEditor, typedInEditor } from '../../shared/src/keys';
import { measure } from '../../shared/src/metrics';
import type { FocusRequest, OutlinePanel, Row } from '../../shared/src/types';

export interface OutlineHandle {
  focus(itemId: string | null): void;
  scrollTo(itemId: string): void;
}

interface Props {
  readonly ref: Ref<OutlineHandle>;
  readonly core: Core;
  readonly rows: readonly Row[];
  readonly outline: OutlinePanel | null;
}

export default function Outline({ ref, core, rows, outline }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scroller.current,
    estimateSize: () => 26,
    getItemKey: index => rows[index]?.item.id ?? index,
    overscan: 8,
  });

  const scrollTo = (itemId: string | null) => {
    const index = rows.findIndex(row =>
      itemId === null ? row.is_editing : row.item.id === itemId,
    );
    if (index >= 0) {
      virtualizer.scrollToIndex(index, { align: 'auto' });
    }
  };

  // The editor takes the focus itself once it is drawn. The outline only brings it on screen
  useImperativeHandle(ref, () => ({ focus: scrollTo, scrollTo }));
  const request = outline?.focus_request ?? null;

  return (
    <div ref={scroller} className="outline">
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map(virtual => {
          const row = rows[virtual.index];
          if (row === undefined) {
            return null;
          }
          return (
            <div
              key={virtual.key}
              data-index={virtual.index}
              ref={virtualizer.measureElement}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                right: 0,
                transform: `translateY(${virtual.start}px)`,
              }}
            >
              <ItemRow core={core} row={row} request={row.is_editing ? request : null} />
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface RowProps {
  readonly core: Core;
  readonly row: Row;
  /** The order of the core to put the cursor into the editor, for the row in edit. */
  readonly request: FocusRequest | null;
}

/** One row. A row the core did not change is the same object, and is not drawn again. */
const ItemRow = memo(function ItemRow({ core, row, request }: RowProps) {
  const id = row.item.id;
  return (
    <>
      {row.date_header !== null && <div className="date-header">{row.date_header}</div>}
      <div
        className={row.is_selected ? 'row selected' : 'row'}
        style={{ paddingLeft: row.indent_px }}
      >
        <span className="bullet" onClick={() => core.send({ CollapseToggle: { item_id: id } })}>
          {row.has_children ? (row.is_collapsed ? '▸' : '▾') : '•'}
        </span>
        <div className="body">
          {row.is_editing ? (
            <Editor core={core} itemId={id} body={row.item.body} request={request} />
          ) : (
            <div
              // On the press, as the web shell does: the editor takes the focus in this very
              // handler, and the focus change of the browser would take it away again
              onMouseDown={event => {
                event.preventDefault();
                core.send({ EditRequested: { item_id: id, place: 'Unplaced' } });
              }}
              dangerouslySetInnerHTML={{ __html: row.body_model }}
            />
          )}
        </div>
      </div>
    </>
  );
});

interface EditorProps {
  readonly core: Core;
  readonly itemId: string;
  readonly body: string;
  readonly request: FocusRequest | null;
}

/**
 * The editor of an item. Its text is the browser's while the user types: the core's body is
 * put in only when the two differ, so that the IME keeps what it is composing.
 */
function Editor({ core, itemId, body, request }: EditorProps) {
  const area = useRef<HTMLTextAreaElement>(null);
  /** The number of the last request this editor followed. */
  const followed = useRef<number | null>(null);
  useLayoutEffect(() => {
    const element = area.current;
    if (element !== null && element.value !== body) {
      element.value = body;
    }
  }, [body]);
  // The focus, on mount and whenever the core asks again
  useLayoutEffect(() => {
    const element = area.current;
    if (element === null) {
      return;
    }
    const mine = request !== null && request.item_id === itemId ? request : null;
    if (mine !== null && mine.seq !== followed.current) {
      followed.current = mine.seq;
      placeCursor(element, mine.cursor);
    } else if (followed.current === null) {
      followed.current = -1;
      placeCursor(element, null);
    }
  }, [itemId, request]);
  return (
    <textarea
      ref={area}
      data-id={itemId}
      rows={1}
      defaultValue={body}
      onKeyDown={event => {
        measure('key');
        pressInEditor(core, itemId, event.currentTarget, event.nativeEvent);
      }}
      onInput={event => typedInEditor(core, itemId, event.currentTarget)}
      onBlur={() => core.send({ EditFinished: { item_id: itemId } })}
    />
  );
}
