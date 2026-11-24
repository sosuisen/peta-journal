import hljs from 'highlight.js/lib/common';
import {
  Fragment,
  memo,
  useLayoutEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
} from 'react';
import type { FocusRequest, Held, Row } from './border/peta-types';
import {
  eventLinkClicked,
  richLinkPage,
  richLinkWeb,
} from './border/peta-types';
import { deferredKey } from './asked';
import { useAsked } from './askedContext';
import { useCore } from './context';
import { checkWikiLink } from './WikiSuggestions';
import type { Core } from './core';
import {
  charsOfUtf16,
  focusWithoutScroll,
  imageFile,
  isImeComposing,
  localTime,
  setCursorChars,
  textareaOffsetAtPoint,
  textBeforePoint,
} from './dom';
import { autoResize } from './layout';
import { startDrag } from './drag';
import { chordOf, editorState } from './keys';
import { useClosesOnOutside } from './menu';

/** The font size of a heading line in its editor, by the level of the heading. */
const HEADING_FONT_SIZES = ['1.6em', '1.4em', '1.2em', '1.1em', '1em'];

interface Props {
  readonly row: Row;
  /** The order of the core to put the cursor into the editor, for the row in edit. */
  readonly request: FocusRequest | null;
  /** True while the core holds the keys for the editor that comes next. */
  readonly holdingKeys: boolean;
}

/**
 * One item of the outline: its controls, its bullet, its editor or its drawn body, and its
 * menu. A row that the core did not change is the same value, and is not drawn again.
 */
export const OutlineItem = memo(function OutlineItem({
  row,
  request,
  holdingKeys,
}: Props) {
  const core = useCore();
  const id = row.item.id;
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  const host = useRef<HTMLDivElement>(null);

  const openMenu = (event: ReactMouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    const button = event.currentTarget.getBoundingClientRect();
    // The button is inside the box, so the box is on the page
    const box = (host.current as HTMLDivElement).getBoundingClientRect();
    setMenuAt({ x: button.left - box.left, y: button.bottom - box.top });
  };
  const menuButton = (
    <button className="item-menu" onClick={openMenu}>
      ⋯
    </button>
  );
  const classes = ['outline-item'];
  if (row.is_selected) classes.push('selected');
  if (row.is_heading) classes.push('heading');
  if (!row.show_bullet) classes.push('hide-bullet');
  const times = taskTimes(row);

  return (
    <app-outline-item>
      <div
        ref={host}
        id={id}
        className={classes.join(' ')}
        data-heading-level={
          row.heading_level > 0 ? row.heading_level : undefined
        }
        style={{ paddingLeft: `${row.indent_px}px` }}
      >
        <div className="item-controls">
          <div className="slot-menu">{row.has_children && menuButton}</div>
          <div className="slot-toggle">
            {/* Keys, so a new child makes a new triangle: the same button restyled from the
                menu would run the transition of its color, and the browser recomputes its
                style on every frame of it */}
            {row.has_children ? (
              <button
                key="toggle"
                className={
                  row.is_collapsed
                    ? 'collapse-toggle collapsed'
                    : 'collapse-toggle'
                }
                onClick={() =>
                  core.send({ kind: 'CollapseToggle', item_id: id })
                }
              >
                <svg
                  className="triangle"
                  viewBox="0 0 10 6"
                  width="10"
                  height="6"
                >
                  <polygon points="0,0 10,0 5,6" fill="currentColor" />
                </svg>
              </button>
            ) : (
              <Fragment key="menu">{menuButton}</Fragment>
            )}
          </div>
        </div>
        <div
          className="item-content"
          style={{
            zoom: row.effective_zoom === 1 ? undefined : row.effective_zoom,
          }}
        >
          <div className="slot-bullet">
            <Bullet row={row} />
          </div>
          <div className="item-body">
            {row.is_broken_transclusion ? (
              <div className="broken-transclusion">
                source not found: {row.item.body}
              </div>
            ) : row.is_editing ? (
              <ItemEditor
                row={row}
                request={request}
                holdingKeys={holdingKeys}
              />
            ) : (
              <ItemView row={row} />
            )}
          </div>
          {times !== null && (
            <div className="task-timestamps" title={times.tooltip}>
              {times.label}
            </div>
          )}
        </div>
      </div>
      {menuAt !== null && (
        <ItemMenu row={row} at={menuAt} close={() => setMenuAt(null)} />
      )}
    </app-outline-item>
  );
});

function Bullet({ row }: { readonly row: Row }) {
  let mark = null;
  if (row.is_transclusion) {
    mark = row.is_collapsed ? (
      <rect
        x="1"
        y="1"
        width="12"
        height="12"
        rx="1"
        fill="#ddd"
        stroke="none"
      />
    ) : (
      <rect
        x="1"
        y="1"
        width="12"
        height="12"
        rx="1"
        fill="none"
        stroke="#ccc"
        strokeWidth="1"
      />
    );
  } else if (row.is_collapsed) {
    mark = <circle cx="7" cy="7" r="6.5" fill="#ddd" stroke="none" />;
  }
  return (
    <span className="bullet">
      <svg viewBox="0 0 14 14" width="14" height="14">
        {mark}
        <circle cx="7" cy="7" r="2.5" fill="#888" stroke="none" />
      </svg>
    </span>
  );
}

/** The label and the tooltip of the times of a task: `HH:MM-` while it runs, its length when done. */
function taskTimes(row: Row): { label: string; tooltip: string } | null {
  if (row.task === null) {
    return null;
  }
  const text = (key: string) => {
    const value = row.item.properties.get(key);
    return value?.kind === 'Text' ? value.value : null;
  };
  const begin = text('begin');
  if (begin === null) {
    return null;
  }
  const from = localTime(begin);
  const end = text('end');
  if (end === null) {
    return { label: `${from}-`, tooltip: `${from}-` };
  }
  return {
    label: duration(begin, end),
    tooltip: `${from} - ${localTime(end)}`,
  };
}

/** How long a task took, as `task_duration` of the core writes it: `1h5m`, `1h`, `5m`, `30s`. */
function duration(begin: string, end: string): string {
  const millis = new Date(end).getTime() - new Date(begin).getTime();
  const seconds = Number.isNaN(millis)
    ? 0
    : Math.floor(Math.max(0, millis) / 1000);
  const [h, m, s] = [
    Math.floor(seconds / 3600),
    Math.floor((seconds % 3600) / 60),
    seconds % 60,
  ];
  if (h === 0) {
    return m === 0 ? `${s}s` : `${m}m`;
  }
  return m === 0 ? `${h}h` : `${h}h${m}m`;
}

function ItemMenu({
  row,
  at,
  close,
}: {
  readonly row: Row;
  readonly at: { x: number; y: number };
  readonly close: () => void;
}) {
  const core = useCore();
  const id = row.item.id;
  useClosesOnOutside('.item-popup-menu', close);
  const act = (action: () => void) => () => {
    action();
    close();
  };
  const copy = (text: string) => void navigator.clipboard.writeText(text);
  return (
    <div
      className="item-popup-menu"
      style={{ left: `${at.x}px`, top: `${at.y}px` }}
    >
      <button onClick={act(() => copy(row.item.body))}>Copy</button>
      <button
        onClick={act(() => {
          copy(row.item.body);
          core.typed({ kind: 'Changed', item_id: id, body: '' });
        })}
      >
        Cut
      </button>
      <button onClick={act(() => copy(id))}>Copy Reference</button>
      <button
        onClick={act(() =>
          core.send({ kind: 'PasteAsTransclusion', item_id: id }),
        )}
      >
        Paste as Transclusion
      </button>
      {row.is_transclusion && (
        <button
          onClick={act(() =>
            core.send({ kind: 'RemoveTransclusion', item_id: id }),
          )}
        >
          Remove Transclusion
        </button>
      )}
      <button
        onClick={act(() => core.send({ kind: 'SendToStash', item_id: id }))}
      >
        Send to Stash
      </button>
      {row.has_own_zoom && (
        <button
          onClick={act(() => core.send({ kind: 'ResetZoom', item_id: id }))}
        >
          Reset Zoom
        </button>
      )}
    </div>
  );
}

function TaskCheckbox({ row }: { readonly row: Row }) {
  const core = useCore();
  const box = useRef<HTMLInputElement>(null);
  const state = row.task?.kind ?? null;
  useLayoutEffect(() => {
    if (box.current !== null) {
      box.current.indeterminate = state === 'Running';
    }
  }, [state]);
  if (state === null) {
    return null;
  }
  return (
    <input
      ref={box}
      type="checkbox"
      className="task-checkbox"
      checked={state === 'Done'}
      onChange={() => {}}
      onClick={event => {
        event.preventDefault();
        event.stopPropagation();
        core.send({ kind: 'TaskCycle', item_id: row.item.id });
      }}
    />
  );
}

/** The marks that the HTML of a body puts on a wikilink and on a hashtag. */
const PAGE_LINKS = 'a[data-wikilink], a.hashtag';

/** The drawn body of an item: the HTML that wasm made of it (ADR 079). */
function ItemView({ row }: { readonly row: Row }) {
  const core = useCore();
  const id = row.item.id;
  const content = useRef<HTMLDivElement>(null);
  const html = row.body_model;

  // What the HTML leaves to the shell (ADR 026): the size of the images, the colours of code
  // The view always draws its content, so the ref is set by its layout effects
  useLayoutEffect(() => {
    const drawn = content.current as HTMLDivElement;
    sizeImages(drawn);
    highlightCode(drawn);
  }, [html]);

  const pressed = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (
      !(event.target instanceof Element) ||
      event.target.closest('a') !== null
    ) {
      return;
    }
    // The editor takes the place of this view, and the focus, in this very handler. The
    // focus change of the browser on the press would take it away again
    event.preventDefault();
    const element = event.currentTarget;
    // The core lines up the letters before the click with the Markdown, and puts the cursor
    // there (ADR 044). Without letters, it goes to the end (ADR 057)
    const before = textBeforePoint(element, event.clientX, event.clientY);
    core.send({
      kind: 'EditRequested',
      item_id: id,
      place:
        before === null
          ? { kind: 'Unplaced' }
          : { kind: 'AfterShownText', value: before },
    });
    const request = core.current().panels.Outline?.focus_request;
    const cursor = request?.item_id === id ? request.cursor : null;
    startDrag(core, id, element, cursor === null ? null : Number(cursor));
  };

  // An image dropped on the drawn item goes at the end of its body: the drawn text and the
  // Markdown do not line up
  const dropped = (event: React.DragEvent<HTMLDivElement>) => {
    const file = imageFile(event.dataTransfer);
    if (file === null) {
      return;
    }
    event.preventDefault();
    core.pasteImage(id, file, [...row.item.body].length);
  };

  return (
    <app-item-view>
      <TaskCheckbox row={row} />
      <div
        ref={content}
        className="item-content"
        dangerouslySetInnerHTML={{ __html: html }}
        onMouseDown={pressed}
        onClick={event => clickedLink(core, event, heldOf(event))}
        onAuxClick={event => {
          // The middle button opens a new view, as Ctrl does
          if (event.button === 1) {
            clickedLink(core, event, { shift: false, ctrl: true, cmd: false });
          }
        }}
        onDrop={dropped}
      />
    </app-item-view>
  );
}

/** The keys held down at a click. */
function heldOf(event: ReactMouseEvent): Held {
  return { shift: event.shiftKey, ctrl: event.ctrlKey, cmd: event.metaKey };
}

/**
 * Opens what a click in a drawn body names. A footnote comes to the middle of the screen. A
 * link goes to the core with the keys `held`, and the core decides how it opens (ADR 083).
 */
function clickedLink(
  core: Core,
  event: ReactMouseEvent<HTMLDivElement>,
  held: Held,
): void {
  // A click lands on an element, never on a text node
  const target = event.target as Element;
  const footnote = target.closest('.footnote-ref a, .footnote-label');
  if (footnote !== null) {
    event.preventDefault();
    event.stopPropagation();
    // flatmarkdown gives both links of a footnote an href of the form #id
    const href = footnote.getAttribute('href') as string;
    document.getElementById(href.slice(1))?.scrollIntoView({ block: 'center' });
    return;
  }
  const page = target.closest(PAGE_LINKS);
  const href = page?.getAttribute('href') ?? null;
  // The address the browser reads from the link, also for a file of the assets folder
  const link = page === null ? target.closest('a') : null;
  const clicked =
    href !== null
      ? richLinkPage(href)
      : link !== null && link.href !== ''
        ? richLinkWeb(link.href)
        : null;
  if (clicked === null) {
    return;
  }
  // The browser would open it too
  event.preventDefault();
  event.stopPropagation();
  core.send(eventLinkClicked(clicked, held));
}

/** Gives each image the size that the fragment of its URL carries (`#w=300&h=240`). */
function sizeImages(content: Element): void {
  for (const image of content.querySelectorAll('img')) {
    // A URL without a fragment gives no sides
    const sides = new URLSearchParams(image.getAttribute('src')?.split('#')[1]);
    for (const [side, name] of [
      ['w', 'width'],
      ['h', 'height'],
    ] as const) {
      const pixels = Number(sides.get(side));
      if (Number.isInteger(pixels) && pixels > 0) {
        image.setAttribute(name, String(pixels));
      }
    }
  }
}

/**
 * Colours each code block that names its language, and shows the name on it. A block
 * without a language is left alone: highlight.js would guess one, and colour plain text.
 */
function highlightCode(content: Element): void {
  for (const code of content.querySelectorAll<HTMLElement>('pre code')) {
    const language = [...code.classList]
      .find(name => name.startsWith('language-'))
      ?.slice('language-'.length);
    const pre = code.parentElement;
    if (language === undefined || pre === null) {
      continue;
    }
    hljs.highlightElement(code);
    if (pre.querySelector('.code-lang-label') === null) {
      const label = document.createElement('div');
      label.className = 'code-lang-label';
      label.textContent = language;
      pre.style.position = 'relative';
      pre.append(label);
    }
  }
}

/**
 * The editor of an item. Its text is the browser's while the user types: the body of the core
 * goes in only when the two differ, so that the IME keeps what it composes.
 */
function ItemEditor({
  row,
  request,
  holdingKeys,
}: {
  readonly row: Row;
  readonly request: FocusRequest | null;
  readonly holdingKeys: boolean;
}) {
  const core = useCore();
  const asked = useAsked();
  const id = row.item.id;
  const body = row.item.body;
  const area = useRef<HTMLTextAreaElement>(null);
  /** The number of the last request this editor followed. */
  const followed = useRef<bigint | null>(null);
  /** A paste has no keys held: the Shift of Ctrl+Shift+V comes from the keys before it. */
  const shiftHeld = useRef(false);

  // The body as the core has it: a paste, an undo and the completion change it from outside
  // The editor always draws its textarea, so the ref is set by its layout effects
  useLayoutEffect(() => {
    const el = area.current as HTMLTextAreaElement;
    if (el.value !== body) {
      el.value = body;
      autoResize(el);
    }
  }, [body]);

  // The focus and the cursor the core asks for, when the editor comes and at each new request
  useLayoutEffect(() => {
    const el = area.current as HTMLTextAreaElement;
    const mine = request !== null && request.item_id === id ? request : null;
    if (mine !== null && mine.seq !== followed.current) {
      followed.current = mine.seq;
      if (mine.cursor !== null) {
        setCursorChars(el, Number(mine.cursor));
      }
      focusWithoutScroll(el);
      autoResize(el);
    } else if (followed.current === null) {
      followed.current = -1n;
      focusWithoutScroll(el);
      autoResize(el);
    }
  }, [id, request]);

  const input = (event: React.FormEvent<HTMLTextAreaElement>) => {
    const el = event.currentTarget;
    const typed = (event.nativeEvent as InputEvent).inputType === 'insertText';
    if (typed) {
      // Each typed text, not each letter: the core closes the openings just typed (ADR 057)
      const cursor = charsOfUtf16(el.value, el.selectionStart);
      core.typed({
        kind: 'Typed',
        item_id: id,
        body: el.value,
        cursor: BigInt(el.selectionStart),
      });
      const shown = core.current().rows.find(r => r.item.id === id)?.item.body;
      if (shown !== undefined && shown !== el.value) {
        el.value = shown;
        setCursorChars(el, cursor);
      }
    } else {
      core.typed({ kind: 'Changed', item_id: id, body: el.value });
    }
    autoResize(el);
    checkWikiLink(core, id, el);
  };

  const keyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const native = event.nativeEvent;
    shiftHeld.current = event.shiftKey;
    // The IME keeps its own keys, as the Enter that confirms a conversion
    if (isImeComposing(native)) {
      return;
    }
    // Keys pressed between Enter and the focus of the next editor belong to that editor
    if (holdingKeys && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      core.send({
        kind: 'KeyHeld',
        value: deferredKey(native),
      });
      return;
    }
    // The keys played after the held ones, into the next editor, wait behind them
    if (asked.keepForReplay(native)) {
      event.preventDefault();
      return;
    }
    const el = event.currentTarget;
    if (event.key === 'Enter' && event.shiftKey && !event.ctrlKey) {
      // Shift+Enter: a break of the line, which Flat Markdown keeps as \n
      event.preventDefault();
      el.setRangeText('\n', el.selectionStart, el.selectionEnd, 'end');
      core.typed({ kind: 'Changed', item_id: id, body: el.value });
      autoResize(el);
      return;
    }
    const chord = chordOf(native);
    if (chord === null) {
      return;
    }
    const taken = core.press({
      kind: 'InEditor',
      item_id: id,
      chord,
      editor: editorState(el, event.key),
    });
    if (taken) {
      event.preventDefault();
      // The outline would take Shift with an arrow as a key of the selection it starts
      event.stopPropagation();
    }
  };

  const pasted = (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const el = event.currentTarget;
    const cursor = charsOfUtf16(el.value, el.selectionStart);
    const file = imageFile(event.clipboardData);
    event.preventDefault();
    if (file !== null) {
      core.pasteImage(id, file, cursor);
      return;
    }
    // Windows puts \r\n on the clipboard
    const text = event.clipboardData
      .getData('text/plain')
      .replace(/\r\n?/g, '\n');
    core.send({
      kind: 'PasteText',
      item_id: id,
      text,
      html: event.clipboardData.getData('text/html'),
      cursor: BigInt(cursor),
      raw: shiftHeld.current,
    });
  };

  // An image dropped on the editor goes in at the drop, as a pasted one at the cursor. Any
  // other drop, as of text, is the browser's
  const dropped = (event: React.DragEvent<HTMLTextAreaElement>) => {
    const file = imageFile(event.dataTransfer);
    if (file === null) {
      return;
    }
    event.preventDefault();
    const at = textareaOffsetAtPoint(
      event.currentTarget,
      event.clientX,
      event.clientY,
    );
    core.pasteImage(id, file, at);
  };

  // A level out of the sizes, as 0 or 6, has no size of its own
  const headingSize: string | undefined = row.heading_line
    ? HEADING_FONT_SIZES[row.heading_level - 1]
    : undefined;

  return (
    <app-item-editor>
      <TaskCheckbox row={row} />
      <textarea
        ref={area}
        id={`item-editor-${id}`}
        rows={1}
        defaultValue={body}
        style={{
          fontSize: headingSize,
          fontWeight: row.heading_line ? 'bold' : undefined,
        }}
        onInput={input}
        onKeyDown={keyDown}
        onKeyUp={event => (shiftHeld.current = event.shiftKey)}
        onPaste={pasted}
        onDrop={dropped}
        onBlur={() => {
          // A Shift let go in another window sends no keyup here
          shiftHeld.current = false;
          core.send({ kind: 'EditFinished', item_id: id });
        }}
        onMouseDown={event => startDrag(core, id, event.currentTarget, null)}
      />
    </app-item-editor>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-outline-item': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'app-item-view': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
      'app-item-editor': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
