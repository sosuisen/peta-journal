import { act, fireEvent, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import { ConfigView, PropertyDefinition } from './border/peta-types';
import Settings from './Settings';
import { draw, shell } from './test/screens';

/** The settings as the core gives them. Each test changes what it needs. */
function config(changes: Partial<ConfigView> = {}): ConfigView {
  return Object.assign(
    new ConfigView('D:/notes', { kind: 'Us' }, false, 'tags/', '', [], null, 0),
    changes,
  );
}

function definition(
  key: string,
  changes: Partial<PropertyDefinition> = {},
): PropertyDefinition {
  return Object.assign(
    new PropertyDefinition(key, 'text', false, false),
    changes,
  );
}

function settings(shown: ConfigView = config()) {
  const ui = shell();
  ui.wasm.push([], [{ kind: 'Config', value: shown }]);
  const drawn = draw(ui, <Settings />);
  const show = (next: ConfigView) =>
    act(() => ui.wasm.push([], [{ kind: 'Config', value: next }]));
  return { ...ui, ...drawn, show };
}

const input = (name: string) => screen.getByLabelText<HTMLInputElement>(name);

// 設定の画面
describe('the settings screen', () => {
  // 設定が届く前は、何も出さない。
  test('nothing shows before the config arrives', () => {
    const { container } = draw(shell(), <Settings />);

    expect(container.innerHTML).toBe('');
  });

  // Markdownのフォルダは読むだけで出す。Browse...で、フォルダを選ぶようコアに頼む。
  test('the Markdown folder is read-only, and Browse... asks the core to choose one', () => {
    const { wasm } = settings();
    const folder = input('Markdown working directory');

    fireEvent.click(screen.getByText('Browse...'));

    expect([folder.value, folder.readOnly]).toEqual(['D:/notes', true]);
    expect(wasm.events).toEqual([{ kind: 'BrowseFolder' }]);
  });

  // 週の数え方は、コアの値を出し、選ぶとコアに渡す。
  test('the week numbering shows the value of the core, and a choice goes to the core', () => {
    const { wasm, show } = settings();
    const select = input('Week numbering');
    const shown = select.value;

    fireEvent.change(select, { target: { value: 'iso' } });
    show(config({ week_numbering: { kind: 'Iso' } }));
    fireEvent.change(select, { target: { value: 'us' } });

    expect([shown, select.value]).toEqual(['us', 'iso']);
    expect(wasm.events).toEqual([
      { kind: 'SetWeekNumbering', value: { kind: 'Iso' } },
      { kind: 'SetWeekNumbering', value: { kind: 'Us' } },
    ]);
  });

  // 論理的なアウトデントは、コアの値を出し、切り替えるとコアに渡す。
  test('the logical outdent shows the value of the core, and a click goes to the core', () => {
    const { wasm, show } = settings();
    const box = input('Logical outdent');
    const shown = box.checked;

    fireEvent.click(box);
    show(config({ logical_outdent: true }));

    expect([shown, box.checked]).toEqual([false, true]);
    expect(wasm.events).toEqual([{ kind: 'SetLogicalOutdent', value: true }]);
  });

  // タグのフォルダとHomeは、コアの値を出し、変えたときにコアに渡す。1文字ごとには渡さない。
  test('the folder of the tags and Home show the core, and go to it on change', () => {
    const { wasm, show } = settings();
    const tags = input('Tag pages directory');
    const home = input('Home');
    const shown = [tags.value, home.value, tags.placeholder, home.placeholder];

    fireEvent.input(tags, { target: { value: 't' } });
    fireEvent.change(tags, { target: { value: 'topics/' } });
    fireEvent.change(home, { target: { value: 'a/Foo' } });
    show(config({ tags_dir: 'later/', home_link: 'b' }));
    fireEvent.change(tags, { target: { value: 'again/' } });

    expect(shown).toEqual(['tags/', '', 'tags/', 'today']);
    expect([tags.value, home.value]).toEqual(['again/', 'b']);
    expect(wasm.events).toEqual([
      { kind: 'SetTagsDir', value: 'topics/' },
      { kind: 'SetHomeLink', value: 'a/Foo' },
      { kind: 'SetTagsDir', value: 'again/' },
    ]);
  });
});

// プロパティの定義
describe('the property definitions', () => {
  const definitions = [
    definition('rating_n', { value_type: 'number', page: true }),
    definition('modified', { value_type: 'datetime', system: true }),
  ];

  const rows = (container: Element) =>
    [...container.querySelectorAll('tbody tr')].map(tr => [
      tr.className,
      ...[...tr.querySelectorAll('td')].map(td => td.textContent),
    ]);

  // 定義は1行ずつ、wasmが決めた名前、型、ページに使えるかを出す。システムのキーには印がつき、消せない。
  test('each definition shows its name from wasm, its type and its use on pages', () => {
    const { container } = settings(
      config({ property_definitions: definitions }),
    );

    expect(rows(container)).toEqual([
      ['', 'shown rating_n', 'number', 'yes', 'Delete'],
      ['system-key', 'shown modified', 'datetime', '', 'system'],
    ]);
  });

  // コアが定義を変えると、名前もwasmに尋ね直す。
  test('the names follow the definitions the core gives', () => {
    const { container, show } = settings(
      config({ property_definitions: definitions }),
    );

    show(config({ property_definitions: [definition('due_d')] }));

    expect(rows(container)).toEqual([
      ['', 'shown due_d', 'text', '', 'Delete'],
    ]);
  });

  // Deleteは、その場でもう一度尋ねる。Noでやめ、Yesで消す。
  test('Delete asks once more in place, No keeps the key and Yes deletes it', () => {
    const { container, wasm } = settings(
      config({ property_definitions: definitions }),
    );

    fireEvent.click(screen.getByText('Delete'));
    const asked = rows(container)[0][4];
    fireEvent.click(screen.getByText('No'));
    const kept = rows(container)[0][4];
    fireEvent.click(screen.getByText('Delete'));
    fireEvent.click(screen.getByText('Yes'));

    expect([asked, kept]).toEqual(['Delete with all values?YesNo', 'Delete']);
    expect(rows(container)[0][4]).toBe('Delete');
    expect(wasm.events).toEqual([
      { kind: 'DeletePropertyDefinition', key: 'rating_n' },
    ]);
  });

  // 新しいキーは、名前と、wasmが決めた型から選んだものを、Addで足す。名前の前後の空白は除く。ページは読み込み直さない。
  test('a new key is added with its name and a type chosen from wasm', () => {
    const { wasm } = settings();
    const name = screen.getByPlaceholderText<HTMLInputElement>('new key');
    const type = name.nextElementSibling as HTMLSelectElement;
    const types = [...type.options].map(o => o.value);

    fireEvent.click(screen.getByText('Add'));
    fireEvent.input(name, { target: { value: ' rating ' } });
    fireEvent.change(type, { target: { value: 'number' } });
    const kept = !fireEvent.submit(name.form as HTMLFormElement);

    expect(types).toEqual(['text', 'number']);
    expect(kept).toBe(true);
    expect(wasm.events).toEqual([
      { kind: 'AddPropertyDefinition', body: '', value_type: 'text' },
      { kind: 'AddPropertyDefinition', body: 'rating', value_type: 'number' },
    ]);
  });

  // コアがキーを受け入れると、名前の欄は空になる。受け入れなければ、残る。
  test('the name is cleared when the core accepts a key, and kept when it does not', () => {
    const { show } = settings();
    const name = screen.getByPlaceholderText<HTMLInputElement>('new key');
    fireEvent.input(name, { target: { value: 'rating' } });

    show(config({ definition_error: 'taken' }));
    const kept = name.value;
    show(config({ definitions_added: 1 }));

    expect([kept, name.value]).toEqual(['rating', '']);
  });

  // 定義の誤りがあれば、出す。
  test('an error of a definition shows', () => {
    const { container, show } = settings();
    const none = container.querySelector('.setting-error');

    show(config({ definition_error: 'rating is taken' }));

    expect(none).toBeNull();
    expect(container.querySelector('.setting-error')?.textContent).toBe(
      'rating is taken',
    );
  });
});
