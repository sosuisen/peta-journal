import {
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react';
import { useCore, usePanel } from './context';
import { highlightCode, sizeImages } from './drawnBody';

/**
 * What the canvas of an atlas asks the dialog of a text to show: the Edit of a card, a group or
 * a callout, Add Card or Add Callout. side is the side of a group title or of the bubble of a
 * callout, and null for a card, which has none.
 */
export interface TextAsked {
  readonly name: string;
  readonly text: string;
  readonly side: string | null;
}

/** The sides a group title or a bubble can take, as the core stores them, with their names */
const SIDES = [
  ['top', 'Top'],
  ['bottom', 'Bottom'],
  ['left', 'Left'],
  ['right', 'Right'],
] as const;

/**
 * The dialog of a text of the atlas: the Markdown in a box of several lines, and a Preview that
 * draws it as a body of a page is drawn. Enter breaks the line. OK, or Ctrl or Command with
 * Enter, gives the side and the text to `done`; Cancel, Escape and a press beside the dialog
 * close it with nothing given, as the confirm dialog of the page closes on Escape.
 */
export default function AtlasTextDialog({
  asked,
  done,
  close,
}: {
  readonly asked: TextAsked;
  readonly done: (side: string | null, text: string) => void;
  readonly close: () => void;
}) {
  const [text, setText] = useState(asked.text);
  const [side, setSide] = useState(asked.side);
  const [previewing, setPreviewing] = useState(false);
  const input = useRef<HTMLTextAreaElement>(null);

  // The cursor goes to the end, where more is written, each time the box comes back
  useLayoutEffect(() => {
    const box = input.current;
    if (box !== null) {
      box.focus();
      box.setSelectionRange(box.value.length, box.value.length);
    }
  }, [previewing]);

  const ok = () => {
    close();
    done(side, text);
  };
  const pressed = (event: ReactKeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (
      event.key === 'Enter' &&
      (event.ctrlKey || event.metaKey) &&
      !event.nativeEvent.isComposing
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
          close();
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
          {asked.name}
        </div>
        <div className="atlas-dialog-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={!previewing}
            onClick={() => setPreviewing(false)}
          >
            Edit
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
          <Preview text={text} />
        ) : (
          <textarea
            ref={input}
            className="atlas-dialog-text"
            placeholder="Text"
            value={text}
            onChange={event => setText(event.target.value)}
          />
        )}
        {side !== null && (
          <div className="atlas-dialog-sides" role="radiogroup">
            {SIDES.map(([value, name]) => (
              <label key={value}>
                <input
                  type="radio"
                  name="atlas-dialog-side"
                  checked={side === value}
                  onChange={() => setSide(value)}
                />
                {name}
              </label>
            ))}
          </div>
        )}
        <div className="atlas-dialog-buttons">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button type="button" className="atlas-dialog-ok" onClick={ok}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}

/** The Markdown drawn as the body of a row of a page, whose links open nothing here */
function Preview({ text }: { readonly text: string }) {
  const core = useCore();
  const tagsDir = usePanel('Config')?.tags_dir ?? '';
  const html = core.bodyHtml(text, tagsDir);
  const content = useRef<HTMLDivElement>(null);
  // What the HTML leaves to the shell, as on a page
  useLayoutEffect(() => {
    const drawn = content.current as HTMLDivElement;
    sizeImages(drawn);
    highlightCode(drawn);
  }, [html]);
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
