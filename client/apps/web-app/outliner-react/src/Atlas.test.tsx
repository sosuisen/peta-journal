import { act, createEvent, fireEvent, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import Atlas from './Atlas';
import {
  AtlasCard,
  AtlasGroup,
  AtlasPlace,
  cardToneLink,
  cardTonePlain,
  Bounds,
  Callout,
  eventLinkClicked,
  groupLookText,
  LinkCard,
  richLinkFile,
  richLinkPage,
  richLinkWeb,
  UnplacedCard,
  type Panel,
} from './border/peta-types';
import { draw, shell } from './test/screens';

const scripts = vi.hoisted(() => ({
  loaded: Promise.resolve(),
}));

vi.mock('./atlasScripts', async importOriginal => ({
  ...(await importOriginal<typeof import('./atlasScripts')>()),
  loadAtlasScripts: () => scripts.loaded,
}));

type Handler = (...args: never[]) => void;

/**
 * The canvas of atlas.js as the tests see it: it keeps what it was given. The world point
 * of a point of the screen is 100 to the left and 50 above it.
 */
class FakeCanvas {
  static made: FakeCanvas[] = [];
  readonly given: [string, unknown][] = [];
  readonly handlers: Record<string, Handler> = {};
  readonly start: number[];
  destroyed = false;
  ready = true;
  target: string | null = null;
  parent: string | null = null;

  constructor(
    readonly host: HTMLElement,
    onOpen: Handler,
    onPlace: Handler,
    onMenu: Handler,
    onView: Handler,
    panX: number,
    panY: number,
    zoom: number,
    readonly options: unknown,
  ) {
    Object.assign(this.handlers, { onOpen, onPlace, onMenu, onView });
    this.start = [panX, panY, zoom];
    FakeCanvas.made.push(this);
  }

  setOnGroup(f: Handler) {
    this.handlers.onGroup = f;
  }
  setOnUngroup(f: Handler) {
    this.handlers.onUngroup = f;
  }
  setOnMove(f: Handler) {
    this.handlers.onMove = f;
  }
  setOnScale(f: Handler) {
    this.handlers.onScale = f;
  }
  setOnGroupTitle(f: Handler) {
    this.handlers.onGroupTitle = f;
  }
  setOnAddCallout(f: Handler) {
    this.handlers.onAddCallout = f;
  }
  setOnCallout(f: Handler) {
    this.handlers.onCallout = f;
  }
  setOnMoveCallout(f: Handler) {
    this.handlers.onMoveCallout = f;
  }
  setOnScaleCallout(f: Handler) {
    this.handlers.onScaleCallout = f;
  }
  setOnFollow(f: Handler) {
    this.handlers.onFollow = f;
  }
  setOnRemove(f: Handler) {
    this.handlers.onRemove = f;
  }
  setOnCard(add: Handler, edit: Handler, remove: Handler, bodyOf: Handler) {
    Object.assign(this.handlers, {
      onAddCard: add,
      onEditCard: edit,
      onRemoveLink: remove,
      bodyOf,
    });
  }
  setOnText(f: Handler) {
    this.handlers.onText = f;
  }
  setCards(json: string) {
    this.given.push(['cards', JSON.parse(json)]);
  }
  setCallouts(json: string) {
    this.given.push(['callouts', JSON.parse(json)]);
  }
  toWorldX(clientX: number) {
    return this.ready ? clientX - 100 : null;
  }
  toWorldY(clientY: number) {
    return this.ready ? clientY - 50 : null;
  }
  dragOverFromList(x: number, y: number, ctrl: boolean, shift: boolean) {
    this.given.push(['over', [x, y, ctrl, shift]]);
  }
  endDragFromList() {
    this.given.push(['end', null]);
  }
  groupTargetFromList(_x: number, _y: number, ctrl: boolean) {
    return ctrl ? this.target : null;
  }
  parentFromList(_x: number, _y: number, ctrl: boolean) {
    return ctrl ? this.parent : null;
  }
  destroy() {
    this.destroyed = true;
  }
}

beforeEach(() => {
  FakeCanvas.made = [];
  scripts.loaded = Promise.resolve();
  Object.assign(window, { AtlasCanvas: FakeCanvas });
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

function card(
  id: string,
  placed: boolean,
  changes: Partial<AtlasCard> = {},
): AtlasCard {
  return Object.assign(
    new AtlasCard(
      id,
      id.toUpperCase(),
      `m/${id}`,
      false,
      placed ? new Bounds(1, 2, 0, 0) : null,
      [],
      [],
      false,
      '',
    ),
    changes,
  );
}

/** The atlas of the folder `dir`, with its canvas made. */
async function atlas(dir: string, ...panels: Panel[]) {
  const ui = shell();
  ui.wasm.push([], panels);
  const drawn = draw(ui, <Atlas dirPath={dir} />);
  await act(() => scripts.loaded);
  const canvas = () => FakeCanvas.made[FakeCanvas.made.length - 1];
  const show = (...more: Panel[]) => act(() => ui.wasm.push([], more));
  return { ...ui, ...drawn, canvas, show };
}

// アトラスのキャンバス
describe('the canvas of an atlas', () => {
  // スクリプトを読んだあとに、キャンバスを作る。ラベルと題名の字は、アトラスの字である。
  test('the canvas is made once the scripts are loaded, with the fonts of the atlas', async () => {
    let load = () => undefined as void;
    scripts.loaded = new Promise(resolve => {
      load = resolve;
    });
    const { container } = draw(shell(), <Atlas dirPath="m/" />);
    const before = FakeCanvas.made.length;

    await act(async () => {
      load();
      await scripts.loaded;
    });

    const made = FakeCanvas.made[0];
    expect(before).toBe(0);
    expect(made.host).toBe(container.querySelector('.atlas-canvas'));
    expect(made.options).toEqual({
      labelFont: {
        family: 'PetaAtlas Label',
        url: expect.stringContaining('NotoSansJP-Regular.ttf') as string,
      },
      titleFont: {
        family: '851 Tegaki Zatsu',
        url: expect.stringContaining(
          '851tegaki_zatsu_normal_0883.ttf',
        ) as string,
      },
    });
  });

  // スクリプトを読めなければ、キャンバスは作らず、そのことを残す。
  test('without the scripts there is no canvas, and the failure is logged', async () => {
    const logged = vi.spyOn(console, 'error').mockImplementation(() => {});
    scripts.loaded = Promise.reject(new Error('offline'));

    draw(shell(), <Atlas dirPath="m/" />);
    await act(async () => {
      await scripts.loaded.catch(() => undefined);
    });

    expect(FakeCanvas.made).toEqual([]);
    expect(logged).toHaveBeenCalled();
  });

  // キャンバスは、そのフォルダで前に残した位置と倍率で始まる。動かすと、フォルダごとに残す。
  test('the canvas starts at the view kept for its folder, and a new view is kept', async () => {
    localStorage.setItem(
      'atlas-views',
      JSON.stringify({ 'm/': { x: 5, y: 6, zoom: 2 } }),
    );
    const { canvas } = await atlas('m/');
    const kept = canvas().start;

    canvas().handlers.onView(7 as never, 8 as never, 3 as never);
    const other = await atlas('n/');

    expect(kept).toEqual([5, 6, 2]);
    expect(other.canvas().start).toEqual([0, 0, 1]);
    expect(JSON.parse(localStorage.getItem('atlas-views') ?? '')).toEqual({
      'm/': { x: 7, y: 8, zoom: 3 },
    });
  });

  // 同じ場所でほかのフォルダのアトラスに移ると、前のキャンバスを壊し、そのフォルダのキャンバスを作る。
  test('another folder in the same place destroys the canvas and makes one of its own', async () => {
    localStorage.setItem(
      'atlas-views',
      JSON.stringify({ 'n/': { x: 1, y: 2, zoom: 3 } }),
    );
    const { canvas, rerender } = await atlas('m/');
    const first = canvas();

    rerender(<Atlas dirPath="n/" />);
    await act(() => scripts.loaded);

    expect(first.destroyed).toBe(true);
    expect(canvas().start).toEqual([1, 2, 3]);
  });

  // 残した位置が読めなければ、真ん中から始める。
  test('an unreadable view starts at the middle', async () => {
    localStorage.setItem('atlas-views', '{');

    const { canvas } = await atlas('m/');

    expect(canvas().start).toEqual([0, 0, 1]);
  });

  // 置いたカードとリンク、吹き出しを、wasmが決めた形で、変わるたびにキャンバスに渡す。
  test('the cards and the callouts go to the canvas whenever they change', async () => {
    const link = new LinkCard(
      'l',
      'x/Foo',
      'Foo',
      cardToneLink(),
      [],
      '',
      new Bounds(0, 0, 0, 0),
      [],
    );
    const { canvas, show } = await atlas(
      'm/',
      { kind: 'AtlasCards', value: [card('a', true)] },
      { kind: 'AtlasLinks', value: [link] },
    );

    show({
      kind: 'AtlasCallouts',
      value: [new Callout('k', 1, 2, 1, 'left', 'hi', [])],
    });

    expect(canvas().given).toEqual([
      ['cards', { cards: ['a'], links: ['l'] }],
      ['callouts', []],
      ['callouts', ['k']],
    ]);
  });

  // 部品がなくなると、キャンバスを壊す。キャンバスができる前になくなれば、作らない。
  test('the canvas is destroyed with the atlas, and not made when the atlas is gone first', async () => {
    const { canvas, unmount } = await atlas('m/');
    unmount();
    const destroyed = canvas().destroyed;
    let load = () => undefined as void;
    scripts.loaded = new Promise(resolve => {
      load = resolve;
    });
    const early = draw(shell(), <Atlas dirPath="m/" />);

    early.unmount();
    await act(async () => {
      load();
      await scripts.loaded;
    });

    expect(destroyed).toBe(true);
    expect(FakeCanvas.made).toHaveLength(1);
  });
});

// キャンバスの操作
describe('what is done on the canvas', () => {
  // ラベルを押すとページを開く。Ctrlならタブ、Shiftなら窓で開く。
  test('a click on a label opens the page, in a tab with Ctrl and in a window with Shift', async () => {
    const { canvas, wasm } = await atlas('m/');
    const open = canvas().handlers.onOpen as (
      path: string,
      tab: boolean,
      window: boolean,
    ) => void;

    open('m/a', false, false);
    open('m/a', true, false);
    open('m/a', true, true);

    expect(wasm.events).toEqual(
      ['Here', 'NewView', 'Browser'].map(kind => ({
        kind: 'GoToPage',
        path: 'm/a',
        open_in: { kind },
      })),
    );
  });

  // カードの1行目のリンクを押すと、ページのリンクと同じく、押していたキーとともにコアに渡す。CtrlはCtrl、ShiftはShiftである。
  test('a click on a link in the text of a card goes to the core as a link of a page does', async () => {
    const { canvas, wasm } = await atlas('m/');
    const follow = canvas().handlers.onFollow as (
      link: { kind: string; url: string },
      tab: boolean,
      window: boolean,
    ) => void;
    const none = { shift: false, ctrl: false, cmd: false };

    follow({ kind: 'Page', url: '/pages/x/a' }, false, false);
    follow({ kind: 'Web', url: 'https://example.com/' }, true, false);
    follow({ kind: 'File', url: '/assets/a.pdf' }, false, true);

    expect(wasm.events).toEqual([
      eventLinkClicked(richLinkPage('/pages/x/a'), none),
      eventLinkClicked(richLinkWeb('https://example.com/'), {
        ...none,
        ctrl: true,
      }),
      eventLinkClicked(richLinkFile('/assets/a.pdf'), { ...none, shift: true }),
    ]);
  });

  // カード、グループ、吹き出しの操作は、どれもコアに渡す。Add CardとEditは、書いたMarkdownを渡す。
  test('the changes of the cards, the groups and the callouts go to the core', async () => {
    const { canvas, wasm } = await atlas('m/');
    const on = canvas().handlers as Record<
      string,
      (...args: unknown[]) => void
    >;

    on.onPlace('a', 1, 2);
    on.onGroup('["a","b"]', 'c', '[{"id":"a","x":1,"y":2}]');
    on.onUngroup('["a"]', '[{"id":"a","x":3,"y":4}]');
    on.onMove('["a","b"]', 'g', '[]');
    on.onScale('a', 2);
    on.onGroupTitle('g', 'top', 'Plan');
    on.onAddCallout(1, 2, 'left', 'hi');
    on.onCallout('k', 'right', 'bye');
    on.onMoveCallout('k', 3, 4);
    on.onScaleCallout('k', 1.5);
    on.onAddCard(5, 6, '[[x/Foo]]');
    on.onEditCard('l', 'note\nmore');
    on.onRemoveLink('l');
    on.onRemove('p');

    expect(wasm.events).toEqual([
      { kind: 'PlaceAtlasCard', card_id: 'a', x: 1, y: 2 },
      {
        kind: 'GroupAtlasCards',
        card_ids: ['a', 'b'],
        target_id: 'c',
        places: [new AtlasPlace('a', 1, 2)],
      },
      {
        kind: 'UngroupAtlasCards',
        card_ids: ['a'],
        places: [new AtlasPlace('a', 3, 4)],
      },
      {
        kind: 'MoveAtlasCards',
        card_ids: ['a', 'b'],
        parent_id: 'g',
        places: [],
      },
      { kind: 'ScaleAtlasCard', card_id: 'a', scale: 2 },
      { kind: 'SetAtlasGroupTitle', group_id: 'g', side: 'top', title: 'Plan' },
      { kind: 'AddAtlasCallout', x: 1, y: 2, side: 'left', text: 'hi' },
      { kind: 'SetAtlasCallout', key: 'k', side: 'right', text: 'bye' },
      { kind: 'MoveAtlasCallout', key: 'k', x: 3, y: 4 },
      { kind: 'ScaleAtlasCallout', key: 'k', scale: 1.5 },
      { kind: 'AddAtlasCard', x: 5, y: 6, text: '[[x/Foo]]' },
      { kind: 'SetAtlasCardText', card_id: 'l', text: 'note\nmore' },
      { kind: 'RemoveAtlasLink', item_id: 'l' },
      { kind: 'RemoveAtlasCard', page_id: 'p' },
    ]);
  });

  // カード、グループ、吹き出しのEditには、その項目の本文を、Markdownのまま出す。ページのカードはページで、ほかは項目で名指す。今の本文である。
  test('the Edit of a card, a group or a callout shows the body of its item as Markdown', async () => {
    const group = new AtlasGroup(
      'g',
      null,
      '[[x/Plan]]',
      groupLookText(),
      0,
      0,
    );
    const page = card('a', true, { text: '[[m/a]]\nmore', groups: [group] });
    const link = new LinkCard(
      'l',
      null,
      'note',
      cardTonePlain(),
      [],
      'note **x**',
      new Bounds(0, 0, 0, 0),
      [],
    );
    const { canvas, show } = await atlas(
      'm/',
      { kind: 'AtlasCards', value: [page] },
      { kind: 'AtlasLinks', value: [link] },
      {
        kind: 'AtlasCallouts',
        value: [new Callout('k', 1, 2, 1, 'left', 'hi', [])],
      },
    );
    const bodyOf = (id: string) =>
      (canvas().handlers.bodyOf as (id: string) => string | undefined)(id);
    const before = ['a', 'l', 'k', 'g', 'none'].map(bodyOf);
    show({
      kind: 'AtlasLinks',
      value: [Object.assign(link, { text: 'note, edited' })],
    });

    expect(before).toEqual([
      '[[m/a]]\nmore',
      'note **x**',
      'hi',
      '[[x/Plan]]',
      undefined,
    ]);
    expect(bodyOf('l')).toBe('note, edited');
  });

  // キャンバスでは、ブラウザのメニューを出さない。キャンバスのメニューを覆うためである。
  test('the menu of the browser does not show on the canvas', async () => {
    const { container } = await atlas('m/');

    const shown = fireEvent.contextMenu(
      container.querySelector('.atlas-canvas') as Element,
    );

    expect(shown).toBe(false);
  });
});

// 横で待つもの: 未追加の項目と、座標のない項目
/** What the canvas asks the dialog of a text to show, and what came back from it */
interface Asked {
  readonly name: string;
  readonly text: string;
  readonly side: string | null;
}

/** The atlas with the dialog of a text opened by its canvas; `done` keeps what OK gave. */
async function dialogOf(asked: Asked) {
  const drawn = await atlas('m/');
  const done: [string | null, string][] = [];
  const open = drawn.canvas().handlers.onText as (
    asked: Asked,
    done: (side: string | null, text: string) => void,
  ) => void;
  act(() => open(asked, (side, text) => done.push([side, text])));
  const input = () => screen.getByRole('textbox');
  return { ...drawn, done, input };
}

// テキストのダイアログ（Edit、Add Card、グループと吹き出しのEdit）
describe('the dialog of a text', () => {
  // キャンバスが頼むと、その名のダイアログを、編集のタブで開く。本文は複数行の入力に出る。
  test('the canvas opens it by its name on the edit tab, with the text in a box of several lines', async () => {
    const { input } = await dialogOf({
      name: 'Edit',
      text: 'one\ntwo',
      side: null,
    });

    expect(screen.getByRole('dialog', { name: 'Edit' })).toBeTruthy();
    expect(input().tagName).toBe('TEXTAREA');
    expect((input() as HTMLTextAreaElement).value).toBe('one\ntwo');
    expect(document.activeElement).toBe(input());
    expect((input() as HTMLTextAreaElement).selectionStart).toBe(7);
    expect(
      screen.getByRole('tab', { name: 'Edit' }).getAttribute('aria-selected'),
    ).toBe('true');
    expect(screen.queryByRole('radio')).toBeNull();
  });

  // OKは書いたものを渡して閉じる。辺のないものは、辺をnullで渡す。
  test('OK gives what is written and closes, with no side for a card', async () => {
    const { done, input } = await dialogOf({
      name: 'Add Card',
      text: '',
      side: null,
    });

    fireEvent.change(input(), { target: { value: 'a\nb' } });
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));

    expect(done).toEqual([[null, 'a\nb']]);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  // Enterは改行を入れるだけで、閉じない。Ctrl+EnterかCommand+EnterがOK、EscapeがCancelである。
  test('Enter only breaks the line; Ctrl or Command with Enter is OK, and Escape is Cancel', async () => {
    const { done, input } = await dialogOf({
      name: 'Edit',
      text: 'x',
      side: null,
    });

    const enter = createEvent.keyDown(input(), { key: 'Enter' });
    fireEvent(input(), enter);
    const open = screen.queryByRole('dialog') !== null;
    fireEvent.keyDown(input(), { key: 'Enter', ctrlKey: true });
    const first = [...done];

    const again = await dialogOf({ name: 'Edit', text: 'y', side: null });
    fireEvent.keyDown(again.input(), { key: 'Enter', metaKey: true });
    const second = [...again.done];

    const third = await dialogOf({ name: 'Edit', text: 'z', side: null });
    fireEvent.keyDown(third.input(), { key: 'Escape' });

    expect(enter.defaultPrevented).toBe(false);
    expect(open).toBe(true);
    expect(first).toEqual([[null, 'x']]);
    expect(second).toEqual([[null, 'y']]);
    expect(third.done).toEqual([]);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  // IMEで変換している間のCtrl+Enterは、OKにならない
  test('Ctrl with Enter while an IME composes is not OK', async () => {
    const { done, input } = await dialogOf({
      name: 'Edit',
      text: 'x',
      side: null,
    });

    fireEvent.keyDown(input(), {
      key: 'Enter',
      ctrlKey: true,
      isComposing: true,
    });

    expect(done).toEqual([]);
    expect(screen.queryByRole('dialog')).not.toBeNull();
  });

  // Cancelと、ダイアログの外の押下は、何も渡さずに閉じる
  test('Cancel and a press beside the dialog close it and give nothing', async () => {
    const first = await dialogOf({ name: 'Edit', text: 'x', side: null });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    const closed = screen.queryByRole('dialog') === null;

    const second = await dialogOf({ name: 'Edit', text: 'y', side: null });
    const beside = document.querySelector('.atlas-dialog-overlay') as Element;
    fireEvent.mouseDown(second.input());
    const kept = screen.queryByRole('dialog') !== null;
    fireEvent.mouseDown(beside);

    expect(closed).toBe(true);
    expect(kept).toBe(true);
    expect([...first.done, ...second.done]).toEqual([]);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  // グループと吹き出しは、辺を選べる。今の辺が選ばれていて、選んだ辺をOKが渡す。
  test('a group or a callout picks its side in the dialog, starting at its own', async () => {
    const { done } = await dialogOf({
      name: 'Edit',
      text: 'Plan',
      side: 'left',
    });
    const picked = (name: string) =>
      screen.getByRole<HTMLInputElement>('radio', { name }).checked;
    const before = ['Top', 'Bottom', 'Left', 'Right'].map(picked);

    fireEvent.click(screen.getByRole('radio', { name: 'Right' }));
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));

    expect(before).toEqual([false, false, true, false]);
    expect(done).toEqual([['right', 'Plan']]);
  });

  // Previewのタブは、書いたMarkdownを、ページの本文と同じに描く。編集に戻っても、書いたものはそのままである。
  test('the Preview tab draws what is written as a body of a page is drawn, and the edit tab keeps it', async () => {
    const { done, input } = await dialogOf({
      name: 'Edit',
      text: 'x',
      side: null,
    });
    fireEvent.change(input(), { target: { value: 'a **b**' } });

    fireEvent.click(screen.getByRole('tab', { name: 'Preview' }));
    const drawn = document.querySelector(
      '.atlas-dialog-preview app-item-view .item-content',
    );
    const html = drawn?.innerHTML;
    const noBox = screen.queryByRole('textbox') === null;
    fireEvent.click(screen.getByRole('tab', { name: 'Edit' }));
    const back = (input() as HTMLTextAreaElement).value;
    fireEvent.keyDown(input(), { key: 'Enter', ctrlKey: true });

    expect(html).toBe('<p>a **b**</p>');
    expect(noBox).toBe(true);
    expect(back).toBe('a **b**');
    expect(done).toEqual([[null, 'a **b**']]);
  });

  // Previewの中のリンクは、押しても開かない
  test('a link in the Preview opens nothing', async () => {
    const { input, wasm } = await dialogOf({
      name: 'Edit',
      text: '',
      side: null,
    });
    fireEvent.change(input(), { target: { value: '<a href="/x">x</a>' } });
    fireEvent.click(screen.getByRole('tab', { name: 'Preview' }));

    const link = document.querySelector('.atlas-dialog-preview a') as Element;
    const click = createEvent.click(link);
    fireEvent(link, click);

    expect(click.defaultPrevented).toBe(true);
    expect(wasm.events).toEqual([]);
  });
});

describe('what waits beside the canvas', () => {
  const CARD_MIME = 'application/x-peta-atlas-card';

  function dataTransfer(types: string[] = []) {
    const data = new Map<string, string>();
    return {
      data,
      types,
      setData: (type: string, value: string) => data.set(type, value),
      getData: (type: string) => data.get(type) ?? '',
      effectAllowed: '',
      dropEffect: '',
    };
  }

  /** jsdom has no DragEvent, so the keys and the place of the pointer are put on by hand. */
  function drag(
    kind: 'dragStart' | 'dragOver' | 'drop',
    target: Element,
    at: {
      x: number;
      y: number;
      ctrl?: boolean;
      meta?: boolean;
      shift?: boolean;
    },
    moved: ReturnType<typeof dataTransfer>,
  ): boolean {
    const event = createEvent[kind](target, { dataTransfer: moved });
    Object.defineProperties(event, {
      clientX: { value: at.x },
      clientY: { value: at.y },
      ctrlKey: { value: at.ctrl ?? false },
      metaKey: { value: at.meta ?? false },
      shiftKey: { value: at.shift ?? false },
    });
    return fireEvent(target, event);
  }

  function waiting(
    id: string,
    added: boolean,
    changes: Partial<UnplacedCard> = {},
  ): UnplacedCard {
    return Object.assign(
      new UnplacedCard(id, id.toUpperCase(), `m/${id}`, false, added),
      changes,
    );
  }

  async function unplaced() {
    const drawn = await atlas(
      'm/',
      { kind: 'AtlasCards', value: [card('a', true), card('b', false)] },
      {
        kind: 'AtlasUnplaced',
        value: [
          waiting('b', false),
          waiting('c', false, { is_dir: true }),
          waiting('d', true),
          waiting('t', true, { path: null }),
        ],
      },
    );
    const listed = () => [
      ...drawn.container.querySelectorAll('.atlas-unplaced .atlas-card'),
    ];
    const host = drawn.container.querySelector('.atlas-canvas') as Element;
    return { ...drawn, listed, host };
  }

  // 横のリストは、ページ（Pages）と、アトラスにない項目（Not on Atlas）の2つのグループに分かれ、間に線がある。フォルダには印がつく。
  test('the list beside the canvas has a group of the pages and a group of what is not on the atlas, with a line between', async () => {
    const { container } = await unplaced();

    const list = container.querySelector('.atlas-unplaced') as HTMLElement;
    expect([...list.children].map(e => e.className)).toEqual([
      'atlas-unplaced-group',
      'atlas-unplaced-separator',
      'atlas-unplaced-group',
    ]);
    const groups = [...container.querySelectorAll('.atlas-unplaced-group')];
    expect(
      groups.map(group => [
        group.querySelector('.atlas-unplaced-title')?.textContent,
        [...group.querySelectorAll('.atlas-card')].map(e => [
          e.className,
          e.textContent,
        ]),
      ]),
    ).toEqual([
      [
        'Pages',
        [
          ['atlas-card', 'B'],
          ['atlas-card folder', 'C'],
        ],
      ],
      [
        'Not on Atlas',
        [
          ['atlas-card', 'D'],
          ['atlas-card', 'T'],
        ],
      ],
    ]);
  });

  // どちらのグループでも、名前を押すと、そのページを開く。何も開かない項目の名前は、押しても何も送らない。
  test('a click on a name of either group opens its page, and a name that opens nothing sends nothing', async () => {
    const { wasm } = await unplaced();

    fireEvent.click(screen.getByText('C'), { ctrlKey: true });
    fireEvent.click(screen.getByText('D'));
    fireEvent.click(screen.getByText('T'));

    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: 'm/c', open_in: { kind: 'NewView' } },
      { kind: 'GoToPage', path: 'm/d', open_in: { kind: 'Here' } },
    ]);
  });

  // 座標のない項目も、リストから引ける。引いたものは、コアが付けた名前で持つ。
  test('an item without a place is dragged from the list by the id the core gave it', async () => {
    const { listed } = await unplaced();
    const moved = dataTransfer();
    const t = listed()[3];
    t.getBoundingClientRect = () =>
      ({ left: 0, top: 0, width: 100, height: 40 }) as DOMRect;

    drag('dragStart', t, { x: 50, y: 20 }, moved);

    expect(JSON.parse(moved.data.get(CARD_MIME) ?? '')).toEqual({
      card_id: 't',
      x: 0.5,
      y: 0.5,
    });
  });

  // リストのカードを引くと、つかんだ場所を、カードの幅と高さの割合で持つ。カードの外は端に寄せる。
  test('a listed card is dragged with the place it was grabbed at, as shares of its size', () =>
    unplaced().then(({ listed }) => {
      const moved = dataTransfer();
      const [b] = listed();
      b.getBoundingClientRect = () =>
        ({ left: 10, top: 20, width: 100, height: 40 }) as DOMRect;

      drag('dragStart', b, { x: 35, y: 30 }, moved);
      const first = JSON.parse(moved.data.get(CARD_MIME) ?? '') as unknown;
      drag('dragStart', b, { x: 500, y: 0 }, moved);

      expect(first).toEqual({ card_id: 'b', x: 0.25, y: 0.25 });
      expect(JSON.parse(moved.data.get(CARD_MIME) ?? '')).toEqual({
        card_id: 'b',
        x: 1,
        y: 0,
      });
      expect(moved.effectAllowed).toBe('move');
    }));

  // リストのカードをキャンバスの上で引くと、キャンバスがグループを示す。CtrlかCommandを押すと、入るグループも示す。Ctrlなしで落とすと、カードの上でもグループには入らず、その場所と、つかんだ場所をコアに渡す。
  test('a listed card over the canvas shows its group there, and a drop without Ctrl gives only the place to the core', async () => {
    const { host, canvas, wasm } = await unplaced();
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ card_id: 'b', x: 0.5, y: 0.25 }));
    canvas().target = 'a';

    drag('dragOver', host, { x: 140, y: 60 }, moved);
    const over = drag('dragOver', host, { x: 150, y: 70, meta: true }, moved);
    drag('dragOver', host, { x: 150, y: 70, ctrl: true, shift: true }, moved);
    const dropped = drag('drop', host, { x: 150, y: 70 }, moved);

    expect([over, dropped, moved.dropEffect]).toEqual([false, false, 'move']);
    expect(canvas().given.slice(-4)).toEqual([
      ['over', [140, 60, false, false]],
      ['over', [150, 70, true, false]],
      ['over', [150, 70, true, true]],
      ['end', null],
    ]);
    expect(wasm.events).toEqual([
      {
        kind: 'DropAtlasCard',
        card_id: 'b',
        x: 50,
        y: 20,
        grab_x: 0.5,
        grab_y: 0.25,
        target_id: null,
        parent_id: null,
      },
    ]);
  });

  // Ctrlを押してドロップすると、1つの操作で、ドロップ先の兄弟になる。カードの上ではそのカードの兄弟に、親のblobの上ではその親の子になる。Ctrlなしでは、どちらにもならない。
  test('a drop with Ctrl makes the card a sibling of what it lands on, in one event', async () => {
    const { host, canvas, wasm } = await unplaced();
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ card_id: 'b', x: 0, y: 0 }));
    canvas().parent = 'g';

    drag('drop', host, { x: 100, y: 50, ctrl: true }, moved);
    canvas().target = 'a';
    drag('drop', host, { x: 100, y: 50, meta: true }, moved);
    drag('drop', host, { x: 100, y: 50 }, moved);

    const dropped = (target_id: string | null, parent_id: string | null) => ({
      kind: 'DropAtlasCard',
      card_id: 'b',
      x: 0,
      y: 0,
      grab_x: 0,
      grab_y: 0,
      target_id,
      parent_id,
    });
    expect(wasm.events).toEqual([
      dropped(null, 'g'),
      dropped('a', null),
      dropped(null, null),
    ]);
  });

  // ShiftとCtrlを押してカードの上にドロップすると、1つの操作で、そのカードの子になる。blobの上では、Ctrlなしと同じで、親は変わらない。Shiftだけでも同じである。
  test('a drop with Shift and Ctrl on a card goes under the card, and on the blob of a parent changes no parent', async () => {
    const { host, canvas, wasm } = await unplaced();
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ card_id: 'b', x: 0, y: 0 }));
    const held = { x: 100, y: 50, ctrl: true, shift: true };
    canvas().parent = 'g';

    drag('drop', host, held, moved);
    canvas().target = 'a';
    drag('drop', host, held, moved);
    drag('drop', host, { x: 100, y: 50, shift: true }, moved);

    const dropped = (parent_id: string | null) => ({
      kind: 'DropAtlasCard',
      card_id: 'b',
      x: 0,
      y: 0,
      grab_x: 0,
      grab_y: 0,
      target_id: null,
      parent_id,
    });
    expect(wasm.events).toEqual([dropped(null), dropped('a'), dropped(null)]);
  });

  // ほかから引いてきた文字は、グループを示さず、落としても何もしない。キャンバスの準備がなければ、落としても何もしない。
  test('text from elsewhere shows no group and drops nothing, as does a canvas not yet ready', async () => {
    const { host, canvas, wasm } = await unplaced();
    const given = canvas().given.length;
    const text = dataTransfer(['text/plain']);
    drag('dragOver', host, { x: 0, y: 0 }, text);
    drag('drop', host, { x: 0, y: 0 }, text);
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ card_id: 'b', x: 0, y: 0 }));
    canvas().ready = false;
    drag('drop', host, { x: 0, y: 0 }, moved);

    expect(canvas().given.length).toBe(given);
    expect(wasm.events).toEqual([]);
  });

  // ボタンの上に移るなどしてキャンバスから離れると、グループの印を消す。
  test('leaving the canvas takes the mark of the group away', async () => {
    const { host, canvas } = await unplaced();

    fireEvent.dragLeave(host);

    expect(canvas().given.slice(-1)).toEqual([['end', null]]);
  });
});

// 上のボタン
describe('the up button', () => {
  // 上のボタンは、wasmが決めた、ひとつ上のフォルダを名乗る。押すと、そこへ行く。
  test('the up button names the folder above, and goes there', async () => {
    const { wasm } = await atlas('m/n/');

    fireEvent.click(screen.getByTitle('Up'));

    expect(screen.getByTitle('Up').textContent).toBe('above m/n/');
    expect(wasm.events).toEqual([{ kind: 'AtlasUp' }]);
  });

  // 一番上のフォルダには、上のボタンがない。
  test('the top folder has no up button', async () => {
    await atlas('');

    expect(screen.queryByTitle('Up')).toBeNull();
  });
});
