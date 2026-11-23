import { useMemo, useState } from 'react';
import type { PropertyDefinition, WeekNumbering } from './border/peta-types';
import { useCore, usePanel } from './context';
import { useSavedOnChange, useShownValue } from './shownValue';

const WEEK_NUMBERINGS: Readonly<Record<string, WeekNumbering['kind']>> = {
  us: 'Us',
  iso: 'Iso',
};

/** The settings screen, ported from the Leptos shell. The core keeps the settings. */
export default function Settings() {
  const core = useCore();
  const config = usePanel('Config');
  const definitions = config?.property_definitions;
  // The names are the rule of the core: a type is added when two keys have the same name
  const names = useMemo(
    () => (definitions === undefined ? [] : core.shownNames(definitions)),
    [core, definitions],
  );
  const types = useMemo(() => core.userValueTypes(), [core]);
  const [newKey, setNewKey] = useState('');
  const [newType, setNewType] = useState(types[0]);
  // The form is cleared when the core accepted a key
  const added = config?.definitions_added;
  const [seen, setSeen] = useState(added);
  if (seen !== added) {
    setSeen(added);
    if (seen !== undefined) {
      setNewKey('');
    }
  }

  if (config === undefined) {
    return null;
  }
  return (
    <app-settings>
      <div className="settings">
        <h2>Settings</h2>
        <div className="setting-group">
          <label className="setting-label" htmlFor="setting-markdown-dir">
            Markdown working directory
          </label>
          <div className="input-with-button">
            <input
              id="setting-markdown-dir"
              type="text"
              className="setting-input"
              value={config.markdown_dir}
              readOnly
            />
            <button
              className="browse-btn"
              onClick={() => core.send({ kind: 'BrowseFolder' })}
            >
              Browse...
            </button>
          </div>
          <p className="setting-description">
            Changes require a server restart to take effect.
          </p>
        </div>
        <div className="setting-group">
          <label className="setting-label" htmlFor="setting-week-numbering">
            Week numbering
          </label>
          <select
            id="setting-week-numbering"
            className="setting-select"
            value={config.week_numbering.kind.toLowerCase()}
            onChange={event =>
              core.send({
                kind: 'SetWeekNumbering',
                value: {
                  kind: WEEK_NUMBERINGS[event.currentTarget.value],
                },
              })
            }
          >
            <option value="us">US (Sunday start)</option>
            <option value="iso">ISO (Monday start)</option>
          </select>
        </div>
        <div className="setting-group">
          <label className="setting-checkbox">
            <input
              type="checkbox"
              checked={config.logical_outdent}
              onChange={event =>
                core.send({
                  kind: 'SetLogicalOutdent',
                  value: event.currentTarget.checked,
                })
              }
            />{' '}
            Logical outdent
          </label>
          <p className="setting-description">
            When outdenting, do not break parent-child relationships.
          </p>
        </div>
        <div className="setting-group">
          <label className="setting-label" htmlFor="setting-tags-dir">
            Tag pages directory
          </label>
          <SavedInput
            id="setting-tags-dir"
            placeholder="tags/"
            value={config.tags_dir}
            save={value => core.send({ kind: 'SetTagsDir', value })}
          />
          <p className="setting-description">
            A hashtag such as #topic links to the page topic under this
            directory. Leave it empty to link to the pages root.
          </p>
        </div>
        <div className="setting-group">
          <label className="setting-label" htmlFor="setting-home">
            Home
          </label>
          <SavedInput
            id="setting-home"
            placeholder="today"
            value={config.home_link}
            save={value => core.send({ kind: 'SetHomeLink', value })}
          />
          <p className="setting-description">
            The page the Home button opens, written as the target of a wiki
            link, such as a/Foo for [[a/Foo]]. Leave it empty to open
            today&apos;s journal.
          </p>
        </div>
        <div className="setting-group">
          <label className="setting-label">Properties</label>
          <table className="property-definitions">
            <thead>
              <tr>
                <th>Key</th>
                <th>Type</th>
                <th>Page</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {config.property_definitions.map((d, index) => (
                <DefinitionRow key={d.key} definition={d} name={names[index]} />
              ))}
            </tbody>
          </table>
          <form
            className="property-definition-form"
            onSubmit={event => {
              event.preventDefault();
              core.send({
                kind: 'AddPropertyDefinition',
                body: newKey.trim(),
                value_type: newType,
              });
            }}
          >
            <input
              type="text"
              className="setting-input"
              placeholder="new key"
              value={newKey}
              onChange={event => setNewKey(event.currentTarget.value)}
            />
            <select
              className="setting-select"
              value={newType}
              onChange={event => setNewType(event.currentTarget.value)}
            >
              {types.map(t => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
            <button type="submit" className="browse-btn">
              Add
            </button>
          </form>
          {config.definition_error !== null && (
            <p className="setting-error">{config.definition_error}</p>
          )}
          <p className="setting-description">
            System keys are managed by the app and cannot be defined again. A
            key can be used on items and, when marked, on pages. Type the name
            alone: the type adds a suffix to it in the Markdown files, such as
            _n for number, and the screen does not show it. Deleting a key
            removes its values from every page and item.
          </p>
        </div>
      </div>
    </app-settings>
  );
}

/** A text setting, sent to the core on its change: when the focus leaves or on Enter. */
function SavedInput({
  id,
  placeholder,
  value,
  save,
}: {
  readonly id: string;
  readonly placeholder: string;
  readonly value: string;
  readonly save: (value: string) => void;
}) {
  const input = useShownValue(value);
  useSavedOnChange(input, save);
  return (
    <input
      ref={input}
      id={id}
      type="text"
      className="setting-input"
      placeholder={placeholder}
    />
  );
}

/**
 * One definition, shown by its name. A system key is marked as such instead of offering any
 * control. A user key has a Delete button that asks once more in place, since the values go
 * with the key.
 */
function DefinitionRow({
  definition,
  name,
}: {
  readonly definition: PropertyDefinition;
  readonly name: string;
}) {
  const core = useCore();
  const [confirming, setConfirming] = useState(false);
  let control;
  if (definition.system) {
    control = <span>system</span>;
  } else if (confirming) {
    control = (
      <span className="definition-confirm">
        Delete with all values?
        <button
          className="definition-delete confirm"
          onClick={() => {
            setConfirming(false);
            core.send({
              kind: 'DeletePropertyDefinition',
              key: definition.key,
            });
          }}
        >
          Yes
        </button>
        <button
          className="definition-delete"
          onClick={() => setConfirming(false)}
        >
          No
        </button>
      </span>
    );
  } else {
    control = (
      <button className="definition-delete" onClick={() => setConfirming(true)}>
        Delete
      </button>
    );
  }
  return (
    <tr className={definition.system ? 'system-key' : undefined}>
      <td>{name}</td>
      <td>{definition.value_type}</td>
      <td>{definition.page ? 'yes' : ''}</td>
      <td className="definition-note">{control}</td>
    </tr>
  );
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-settings': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
