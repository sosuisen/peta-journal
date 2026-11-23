import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type Dispatch,
  type DragEvent,
  type KeyboardEvent,
  type RefObject,
  type SetStateAction,
} from 'react';
import type { FreeKey, PageProperty, PageView } from './border/peta-types';
import { useCore, usePanel } from './context';
import {
  isImeComposing,
  localMinutes,
  localMinutesToUtc,
  nowLocalMinutes,
} from './dom';
import { openInOf } from './keys';

/**
 * A row added with `+` and not yet saved. It becomes a property once it has a key and a
 * value. The page keeps it, so the rows stay while the core draws the page again.
 */
export interface Draft {
  readonly id: number;
  readonly key: string;
  /** The type of the picked key; empty before a key is picked. */
  readonly valueType: string;
}

interface Props {
  readonly page: PageView;
  readonly drafts: readonly Draft[];
  readonly setDrafts: Dispatch<SetStateAction<readonly Draft[]>>;
}

/**
 * The property rows between the title and the items of a doc page, ported from the Leptos
 * shell. `+` adds a row whose key is picked from the free keys, and `×` removes a property.
 * Date-times are shown and typed in local time to the minute, and sent as UTC.
 */
export default function PageProperties({ page, drafts, setDrafts }: Props) {
  const core = useCore();
  const nextId = useRef(1);
  const remove = (id: number) => setDrafts(ds => ds.filter(d => d.id !== id));
  const add = () => {
    const id = nextId.current;
    nextId.current += 1;
    setDrafts(ds => [...ds, { id, key: '', valueType: '' }]);
  };
  const pick = (id: number, key: string) => {
    // No key has no type, and its row shows no input
    const valueType = page.free_keys.find(k => k.key === key)?.value_type ?? '';
    setDrafts(ds => ds.map(d => (d.id === id ? { id, key, valueType } : d)));
  };
  const commit = (draft: Draft, value: string) => {
    core.send({
      kind: 'SetPageProperty',
      key: draft.key,
      value_type: draft.valueType,
      value,
    });
    remove(draft.id);
  };

  return (
    <div className="page-properties">
      {page.properties.map(property => (
        <PropertyRow key={property.key} property={property} />
      ))}
      {drafts.map(draft => (
        <DraftRow
          key={draft.id}
          draft={draft}
          freeKeys={page.free_keys}
          pick={key => pick(draft.id, key)}
          commit={value => commit(draft, value)}
          discard={() => remove(draft.id)}
        />
      ))}
      <button className="property-add" title="Add property" onClick={add}>
        +
      </button>
    </div>
  );
}

function PropertyRow({ property }: { readonly property: PageProperty }) {
  const core = useCore();
  const { key } = property;
  let shown;
  if (property.is_list) {
    shown = <TagList propertyKey={key} tags={property.values} />;
  } else if (property.editable) {
    shown = (
      <ValueInput
        valueType={property.value_type}
        value={property.value}
        save={value =>
          core.send({
            kind: 'SetPageProperty',
            key,
            value_type: property.value_type,
            value,
          })
        }
        draft={false}
      />
    );
  } else {
    // The same input as an editable date-time, so the browser shows both dates alike
    shown = (
      <input
        type="datetime-local"
        className="property-value property-readonly"
        readOnly
        value={localMinutes(property.value)}
      />
    );
  }
  return (
    <div className="property-row">
      <span className="property-key">{property.name}</span>
      {shown}
      {property.removable && (
        <button
          className="property-remove"
          title="Remove"
          onClick={() => core.send({ kind: 'DeletePageProperty', key })}
        >
          ×
        </button>
      )}
    </div>
  );
}

function DraftRow({
  draft,
  freeKeys,
  pick,
  commit,
  discard,
}: {
  readonly draft: Draft;
  readonly freeKeys: readonly FreeKey[];
  readonly pick: (key: string) => void;
  readonly commit: (value: string) => void;
  readonly discard: () => void;
}) {
  let input = null;
  if (draft.valueType === 'tags' || draft.valueType === 'list') {
    // The first tags make the property; the row is then gone
    input = <TagList propertyKey={draft.key} tags={[]} added={discard} open />;
  } else if (draft.valueType !== '') {
    // A new date-time starts at now, so Enter alone accepts it
    const initial =
      draft.valueType === 'datetime'
        ? localMinutesToUtc(nowLocalMinutes())
        : '';
    input = (
      <ValueInput
        key={draft.key}
        valueType={draft.valueType}
        value={initial}
        save={commit}
        draft
      />
    );
  }
  return (
    <div className="property-row draft">
      <select
        className="property-key-select"
        value={draft.key}
        onChange={event => pick(event.currentTarget.value)}
      >
        <option value="">key</option>
        {freeKeys.map(k => (
          <option key={k.key} value={k.key}>
            {k.name}
          </option>
        ))}
      </select>
      {input}
      {/* The press is stopped, so the input keeps the focus, and its blur does not save
          the row that is being discarded */}
      <button
        className="property-remove"
        title="Remove"
        onMouseDown={event => event.preventDefault()}
        onClick={discard}
      >
        ×
      </button>
    </div>
  );
}

interface ValueProps {
  readonly valueType: string;
  readonly value: string;
  readonly save: (value: string) => void;
  /** A new row: it takes the focus, and saves a value as it is on Enter or a blur. */
  readonly draft: boolean;
}

/** The input of a value, by the type of its key. */
function ValueInput(props: ValueProps) {
  const { valueType, value, save, draft } = props;
  if (valueType === 'checkbox') {
    return (
      <input
        type="checkbox"
        className="property-checkbox"
        autoFocus={draft}
        checked={value === 'true'}
        onChange={event => save(event.currentTarget.checked ? 'true' : 'false')}
      />
    );
  }
  if (valueType === 'datetime') {
    return <DateTimeInput {...props} />;
  }
  return <TextInput {...props} />;
}

const TEXT_INPUTS: Readonly<
  Record<string, { type: string; placeholder: string }>
> = {
  number: { type: 'number', placeholder: '' },
  date: { type: 'date', placeholder: '' },
  tags: { type: 'text', placeholder: 'a, b' },
  list: { type: 'text', placeholder: 'a, b' },
  bounds: { type: 'text', placeholder: 'x, y, width, height' },
};

/**
 * A text-like value. A saved one is saved on its change: when the focus leaves or on Enter.
 * A new one is saved on Enter and on a blur when it is not empty.
 */
function TextInput({ valueType, value, save, draft }: ValueProps) {
  const input = useShownValue(value);
  // The change of the DOM, not the one of React, which comes on every letter
  useEffect(() => {
    const el = input.current as HTMLInputElement;
    if (draft) {
      return;
    }
    const changed = () => save(el.value);
    el.addEventListener('change', changed);
    return () => el.removeEventListener('change', changed);
  }, [input, draft, save]);
  const leave = (typed: string) => {
    if (draft && typed !== '') {
      save(typed);
    }
  };
  const { type, placeholder } = TEXT_INPUTS[valueType] ?? {
    type: 'text',
    placeholder: '',
  };
  return (
    <input
      ref={input}
      type={type}
      className="property-value"
      placeholder={placeholder}
      autoFocus={draft}
      onKeyDown={event => {
        if (draft && isEnter(event)) {
          event.preventDefault();
          leave(event.currentTarget.value);
        }
      }}
      onBlur={event => leave(event.currentTarget.value)}
    />
  );
}

/**
 * A date-time in local time to the minute, sent as UTC. It is saved only when the focus
 * leaves or on Enter, not on change: a save draws the page again, and that would close the
 * picker after its first click.
 */
function DateTimeInput({ value, save, draft }: ValueProps) {
  const shown = localMinutes(value);
  const input = useShownValue(shown);
  const leave = (local: string) => {
    if (local !== '' && (draft || local !== shown)) {
      save(localMinutesToUtc(local));
    }
  };
  return (
    <input
      ref={input}
      type="datetime-local"
      className="property-value"
      autoFocus={draft}
      onKeyDown={event => {
        if (isEnter(event)) {
          event.preventDefault();
          leave(event.currentTarget.value);
        }
      }}
      onBlur={event => leave(event.currentTarget.value)}
    />
  );
}

function isEnter(event: KeyboardEvent): boolean {
  return event.key === 'Enter' && !isImeComposing(event.nativeEvent);
}

/**
 * Writes the value of the core into the input when it changes. The input keeps its own
 * value in between, so typing is never written over.
 */
function useShownValue(value: string): RefObject<HTMLInputElement | null> {
  const input = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    const el = input.current as HTMLInputElement;
    el.value = value;
  }, [value]);
  return input;
}

/** Where a tag dropped on a chip lands. */
interface DropAt {
  readonly tag: string;
  readonly after: boolean;
}

/**
 * A list value as tag chips. A chip opens its tag page, its `×` removes the tag, and `+`
 * opens an input that adds the typed tags on Enter or when the focus leaves. A new row
 * (`open`) starts with that input open. A chip dragged onto another one moves the tag
 * before or after it.
 */
function TagList({
  propertyKey,
  tags,
  added = () => undefined,
  open = false,
}: {
  readonly propertyKey: string;
  readonly tags: readonly string[];
  /** Called after tags were added: a new row is then gone. */
  readonly added?: () => void;
  readonly open?: boolean;
}) {
  const core = useCore();
  // Tag pages live where hashtags link to, so both reach the same page
  const tagsDir = usePanel('Config')?.tags_dir ?? '';
  const [adding, setAdding] = useState(open);
  // Enter and Escape take the input away, and a blur that comes after must not add again
  const addingNow = useRef(open);
  const [dragged, setDragged] = useState<string | null>(null);
  const [dropAt, setDropAt] = useState<DropAt | null>(null);

  const setOpen = (next: boolean) => {
    addingNow.current = next;
    setAdding(next);
  };
  const add = (typed: string) => {
    if (!addingNow.current) {
      return;
    }
    setOpen(false);
    if (typed.trim() !== '') {
      core.send({ kind: 'AddPageTags', key: propertyKey, typed });
      added();
    }
  };
  const endDrag = () => {
    setDragged(null);
    setDropAt(null);
  };

  return (
    <div className="property-tags">
      {tags.map(tag => {
        const href = core.tagUrl(tagsDir, tag);
        let className = 'property-tag';
        if (dragged === tag) {
          className += ' dragging';
        }
        if (dropAt?.tag === tag) {
          className += dropAt.after ? ' drop-after' : ' drop-before';
        }
        return (
          <span
            key={tag}
            className={className}
            draggable
            onDragStart={event => {
              // Firefox starts no drag without data
              event.dataTransfer.setData('text/plain', `#${tag}`);
              event.dataTransfer.effectAllowed = 'move';
              setDragged(tag);
            }}
            onDragOver={event => {
              // Text or a card dragged from elsewhere is not a tag of this list
              if (dragged === null) {
                return;
              }
              event.preventDefault();
              event.dataTransfer.dropEffect = 'move';
              setDropAt({ tag, after: dropsAfter(event) });
            }}
            onDrop={event => {
              if (dragged === null) {
                return;
              }
              event.preventDefault();
              core.send({
                kind: 'MovePageTag',
                key: propertyKey,
                tag: dragged,
                target: tag,
                after: dropsAfter(event),
              });
              endDrag();
            }}
            // The next dragover marks it again when the pointer only moved onto the link
            onDragLeave={() => setDropAt(null)}
            onDragEnd={endDrag}
          >
            {/* The link would drag its URL instead of the chip */}
            <a
              className="property-tag-link"
              href={href}
              draggable={false}
              onClick={event => {
                // A click with a key held is left to the browser, for a new tab or window
                if (openInOf(event.nativeEvent).kind !== 'Here') {
                  return;
                }
                event.preventDefault();
                core.go(href);
              }}
            >
              {`#${tag}`}
            </a>
            <button
              className="property-tag-remove"
              title="Remove tag"
              onClick={() =>
                core.send({ kind: 'RemovePageTag', key: propertyKey, tag })
              }
            >
              ×
            </button>
          </span>
        );
      })}
      {adding ? (
        <input
          type="text"
          className="property-tag-input"
          placeholder="tag"
          autoFocus
          onKeyDown={event => {
            if (isEnter(event)) {
              event.preventDefault();
              add(event.currentTarget.value);
            } else if (event.key === 'Escape') {
              setOpen(false);
            }
          }}
          onBlur={event => add(event.currentTarget.value)}
        />
      ) : (
        <button
          className="property-tag-add"
          title="Add tag"
          onClick={() => setOpen(true)}
        >
          +
        </button>
      )}
    </div>
  );
}

/** Whether a tag dropped at the pointer goes after the chip: it does on its right half. */
function dropsAfter(event: DragEvent<HTMLElement>): boolean {
  const rect = event.currentTarget.getBoundingClientRect();
  return event.clientX > rect.left + rect.width / 2;
}
