import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react';
import './atlas-dialog.css';

// The dialog of a text of an atlas: the Edit of a card, a group or a callout, Add Card, Add
// Callout, and View Text on a canvas that is not editable. It reads nothing of the app: what it
// shows and what it does come in through its props, so a page outside the app, such as a static
// site, can draw it as it is.

/** `edit` changes the text; `view` only shows it, with Cancel alone */
export type AtlasDialogMode = 'edit' | 'view';

/**
 * The page names offered for the `[[wiki link]]` the cursor is in, and what the dialog tells
 * of the typing. The app gives them from its core, as for the editor of a page.
 */
export interface LinkSuggestions {
  /** The names to show, where on the screen, and the selected one; null for none */
  readonly shown: {
    readonly titles: readonly string[];
    readonly selected: number;
    readonly x: number;
    readonly y: number;
  } | null;
  /** The text or the cursor of the box changed */
  typed(box: HTMLTextAreaElement): void;
  /** An arrow moves the selected name; Escape closes the names */
  key(key: 'ArrowUp' | 'ArrowDown' | 'Escape'): void;
  /**
   * The text with `title` put into the link at the UTF-16 place `cursor`, and the UTF-16
   * place after it; undefined outside a link.
   */
  picked(
    text: string,
    cursor: number,
    title: string,
  ): { readonly text: string; readonly cursor: number } | undefined;
  /** The names are done with: after a pick, and when the dialog goes */
  close(): void;
  /**
   * What a text typed from `before` to `text` becomes with the cursor at the UTF-16 place
   * `cursor`: an opening just typed, as `[[`, with its closing added. Undefined when it stays
   * as it is. Asked only when something was typed in, not deleted.
   */
  closed(before: string, text: string, cursor: number): string | undefined;
}

export interface AtlasDialogProps {
  readonly name: string;
  readonly mode: AtlasDialogMode;
  /** The Markdown of the text */
  readonly text: string;
  /**
   * The side of a group title or of the bubble of a callout to start at, picked in the edit
   * mode; null or absent for a card, which has none. The view mode shows no side.
   */
  readonly side?: string | null;
  /** The HTML the Preview draws for a text, as the body of a page is drawn */
  readonly previewHtml: (text: string) => string;
  /** What the Preview does to its HTML once drawn, as sizing its images */
  readonly drawn?: (content: HTMLElement) => void;
  /** The page names for `[[`, in the edit mode; none offers no names */
  readonly links?: LinkSuggestions;
  /** OK of the edit mode, with the side picked (null without one) and the text */
  readonly onOk?: (side: string | null, text: string) => void;
  /** Cancel, Escape and a press beside the dialog */
  readonly onCancel: () => void;
}

/** The sides a group title or a bubble can take, as the core stores them, with their names */
const SIDES = [
  ['top', 'Top'],
  ['bottom', 'Bottom'],
  ['left', 'Left'],
  ['right', 'Right'],
] as const;

/**
 * The dialog of a text: the Markdown in a box of several lines, and a Preview that draws it.
 * Enter breaks the line. OK, or Ctrl or Command with Enter, gives the side and the text to
 * onOk; Cancel, Escape and a press beside the dialog call onCancel, as the confirm dialog of
 * the page closes on Escape.
 */
export default function AtlasDialog({
  name,
  mode,
  text: start,
  side: startSide,
  previewHtml,
  drawn,
  links,
  onOk,
  onCancel,
}: AtlasDialogProps) {
  const editing = mode === 'edit';
  const [text, setText] = useState(start);
  const [side, setSide] = useState(editing ? (startSide ?? null) : null);
  const [previewing, setPreviewing] = useState(false);
  // Where the cursor goes when the box comes back: the end, or after a picked name
  const cursor = useRef(start.length);
  const input = useRef<HTMLTextAreaElement>(null);
  const shown = editing ? (links?.shown ?? null) : null;

  useLayoutEffect(() => {
    const box = input.current;
    if (box !== null) {
      box.focus();
      box.setSelectionRange(cursor.current, cursor.current);
    }
  }, [previewing, text]);

  // The names go with the dialog, whichever links it was last given
  const latest = useRef(links);
  useEffect(() => {
    latest.current = links;
  });
  useEffect(() => () => latest.current?.close(), []);

  const ok = () => {
    if (editing) {
      onOk?.(side, text);
    }
  };
  const pick = (title: string) => {
    const box = input.current;
    const picked =
      box === null ? undefined : links?.picked(text, box.selectionStart, title);
    links?.close();
    if (picked !== undefined) {
      cursor.current = picked.cursor;
      setText(picked.text);
    }
  };
  const pressed = (event: ReactKeyboardEvent) => {
    const composing = event.nativeEvent.isComposing;
    if (shown !== null && shown.titles.length > 0 && !composing) {
      if (
        event.key === 'ArrowUp' ||
        event.key === 'ArrowDown' ||
        event.key === 'Escape'
      ) {
        event.preventDefault();
        event.stopPropagation();
        links?.key(event.key);
        return;
      }
      if (event.key === 'Enter' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        pick(shown.titles[shown.selected] ?? shown.titles[0]);
        return;
      }
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      onCancel();
    } else if (
      event.key === 'Enter' &&
      (event.ctrlKey || event.metaKey) &&
      !composing
    ) {
      event.preventDefault();
      ok();
    }
  };

  return (
    <div
      className="atlas-dialog-overlay"
      onMouseDown={event => {
        if (event.target === event.currentTarget) {
          onCancel();
        }
      }}
      onKeyDown={pressed}
    >
      <div
        className="atlas-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="atlas-dialog-name"
      >
        <div id="atlas-dialog-name" className="atlas-dialog-name">
          {name}
        </div>
        <div className="atlas-dialog-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={!previewing}
            onClick={() => setPreviewing(false)}
          >
            {editing ? 'Edit' : 'Text'}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={previewing}
            onClick={() => setPreviewing(true)}
          >
            Preview
          </button>
        </div>
        {previewing ? (
          <Preview html={previewHtml(text)} drawn={drawn} />
        ) : (
          <textarea
            ref={input}
            className="atlas-dialog-text"
            placeholder={editing ? 'Text' : undefined}
            readOnly={!editing}
            value={text}
            onChange={event => {
              const box = event.target;
              const typedIn =
                (event.nativeEvent as InputEvent).inputType === 'insertText';
              const closed = typedIn
                ? links?.closed(text, box.value, box.selectionStart)
                : undefined;
              cursor.current = box.selectionStart;
              setText(closed ?? box.value);
              links?.typed(box);
            }}
            onSelect={event => {
              cursor.current = event.currentTarget.selectionStart;
              links?.typed(event.currentTarget);
            }}
          />
        )}
        {shown !== null && shown.titles.length > 0 && !previewing && (
          <div
            className="atlas-dialog-suggestions"
            style={{ left: `${shown.x}px`, top: `${shown.y}px` }}
          >
            {shown.titles.map((title, i) => (
              <div
                key={`${i}:${title}`}
                className={
                  i === shown.selected
                    ? 'atlas-dialog-suggestion selected'
                    : 'atlas-dialog-suggestion'
                }
                onMouseDown={event => {
                  // The box keeps the focus and its cursor
                  event.preventDefault();
                  event.stopPropagation();
                  pick(title);
                }}
              >
                {title}
              </div>
            ))}
          </div>
        )}
        {side !== null && (
          <div className="atlas-dialog-sides" role="radiogroup">
            {SIDES.map(([value, label]) => (
              <label key={value}>
                <input
                  type="radio"
                  name="atlas-dialog-side"
                  checked={side === value}
                  onChange={() => setSide(value)}
                />
                {label}
              </label>
            ))}
          </div>
        )}
        <div className="atlas-dialog-buttons">
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
          {editing && (
            <button type="button" className="atlas-dialog-ok" onClick={ok}>
              OK
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * The HTML drawn as the body of a row of a page: inside `app-item-view` and `.item-content`,
 * which the stylesheet of the page body styles. Its links open nothing here.
 */
function Preview({
  html,
  drawn,
}: {
  readonly html: string;
  readonly drawn?: (content: HTMLElement) => void;
}) {
  const content = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    drawn?.(content.current as HTMLDivElement);
  }, [html, drawn]);
  return (
    <div className="atlas-dialog-preview" role="tabpanel">
      <app-item-view>
        <div
          ref={content}
          className="item-content"
          onClick={event => event.preventDefault()}
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </app-item-view>
    </div>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-item-view': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
