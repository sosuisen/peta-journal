import { act, createEvent, fireEvent, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import Atlas from './Atlas';
import {
  AtlasCard,
  cardToneLink,
  Bounds,
  Callout,
  eventLinkClicked,
  LinkCard,
  richLinkFile,
  richLinkPage,
  richLinkWeb,
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
  setOnShowText(f: Handler) {
    this.handlers.onShowText = f;
  }
  setOnLink(add: Handler, set: Handler, remove: Handler, query: Handler) {
    Object.assign(this.handlers, {
      onAddLink: add,
      onSetLink: set,
      onRemoveLink: remove,
      onLinkQuery: query,
    });
  }
  setCards(json: string) {
    this.given.push(['cards', JSON.parse(json)]);
  }
  setCallouts(json: string) {
    this.given.push(['callouts', JSON.parse(json)]);
  }
  setLinkSuggestions(json: string) {
    this.given.push(['suggestions', JSON.parse(json)]);
  }
  toWorldX(clientX: number) {
    return this.ready ? clientX - 100 : null;
  }
  toWorldY(clientY: number) {
    return this.ready ? clientY - 50 : null;
  }
  dragOverFromList(x: number, y: number, ctrl: boolean) {
    this.given.push(['over', [x, y, ctrl]]);
  }
  endDragFromList() {
    this.given.push(['end', null]);
  }
  groupTargetFromList(_x: number, _y: number, ctrl: boolean) {
    return ctrl ? this.target : null;
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

  // 置いたカードとリンク、吹き出し、リンクの候補を、wasmが決めた形で、変わるたびにキャンバスに渡す。
  test('the cards, the callouts and the link suggestions go to the canvas whenever they change', async () => {
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
      value: [new Callout('k', 1, 2, 1, 'left', 'hi')],
    });
    show({ kind: 'AtlasLinkSuggestions', value: ['Foo'] });

    expect(canvas().given).toEqual([
      ['cards', { cards: ['a'], links: ['l'] }],
      ['callouts', []],
      ['suggestions', []],
      ['callouts', ['k']],
      ['suggestions', ['Foo']],
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

  // カード、グループ、吹き出し、リンクの操作は、どれもコアに渡す。
  test('the changes of the cards, the groups, the callouts and the links go to the core', async () => {
    const { canvas, wasm } = await atlas('m/');
    const on = canvas().handlers as Record<
      string,
      (...args: unknown[]) => void
    >;

    on.onPlace('a', 1, 2);
    on.onGroup('["a","b"]', 'c');
    on.onUngroup('["a"]');
    on.onScale('a', 2);
    on.onGroupTitle('g', 'top', 'Plan');
    on.onAddCallout(1, 2, 'left', 'hi');
    on.onCallout('k', 'right', 'bye');
    on.onMoveCallout('k', 3, 4);
    on.onScaleCallout('k', 1.5);
    on.onAddLink(5, 6, 'x/Foo');
    on.onSetLink('l', 'x/Bar');
    on.onRemoveLink('l');
    on.onLinkQuery('Fo');

    expect(wasm.events).toEqual([
      { kind: 'PlaceAtlasCard', card_id: 'a', x: 1, y: 2 },
      { kind: 'GroupAtlasCards', card_ids: ['a', 'b'], target_id: 'c' },
      { kind: 'UngroupAtlasCards', card_ids: ['a'] },
      { kind: 'ScaleAtlasCard', card_id: 'a', scale: 2 },
      { kind: 'SetAtlasGroupTitle', group_id: 'g', side: 'top', title: 'Plan' },
      { kind: 'AddAtlasCallout', x: 1, y: 2, side: 'left', text: 'hi' },
      { kind: 'SetAtlasCallout', key: 'k', side: 'right', text: 'bye' },
      { kind: 'MoveAtlasCallout', key: 'k', x: 3, y: 4 },
      { kind: 'ScaleAtlasCallout', key: 'k', scale: 1.5 },
      { kind: 'AddAtlasLink', x: 5, y: 6, target: 'x/Foo' },
      { kind: 'SetAtlasLink', item_id: 'l', target: 'x/Bar' },
      { kind: 'RemoveAtlasLink', item_id: 'l' },
      { kind: 'SearchAtlasLinks', query: 'Fo' },
    ]);
  });

  // カードを右クリックすると、その場所にメニューが出る。Remove from Atlasで、カードをキャンバスから外す。
  test('the menu of a card opens where it was clicked, and takes the card off the canvas', async () => {
    const { canvas, wasm, container } = await atlas('m/');

    act(() => canvas().handlers.onMenu('a' as never, 30 as never, 40 as never));
    const menu = container.querySelector('.atlas-menu') as HTMLElement;
    const place = [menu.style.left, menu.style.top];
    fireEvent.click(screen.getByText('Remove from Atlas'));

    expect(place).toEqual(['30px', '40px']);
    expect(container.querySelector('.atlas-menu')).toBeNull();
    expect(wasm.events).toEqual([{ kind: 'RemoveAtlasCard', page_id: 'a' }]);
  });

  // メニューは、外のクリックとEscapeで、何もせずに閉じる。
  test('the menu closes on a click outside and on Escape', async () => {
    const { canvas, wasm, container } = await atlas('m/');
    const open = () =>
      act(() => canvas().handlers.onMenu('a' as never, 0 as never, 0 as never));

    open();
    fireEvent.click(document.body);
    const outside = container.querySelector('.atlas-menu');
    open();
    fireEvent.keyDown(document, { key: 'Escape' });

    expect([outside, container.querySelector('.atlas-menu')]).toEqual([
      null,
      null,
    ]);
    expect(wasm.events).toEqual([]);
  });

  // キャンバスでは、ブラウザのメニューを出さない。カードのメニューを覆うためである。
  test('the menu of the browser does not show on the canvas', async () => {
    const { container } = await atlas('m/');

    const shown = fireEvent.contextMenu(
      container.querySelector('.atlas-canvas') as Element,
    );

    expect(shown).toBe(false);
  });
});

// 置いていないカード
describe('the unplaced cards', () => {
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
    at: { x: number; y: number; ctrl?: boolean; meta?: boolean },
    moved: ReturnType<typeof dataTransfer>,
  ): boolean {
    const event = createEvent[kind](target, { dataTransfer: moved });
    Object.defineProperties(event, {
      clientX: { value: at.x },
      clientY: { value: at.y },
      ctrlKey: { value: at.ctrl ?? false },
      metaKey: { value: at.meta ?? false },
    });
    return fireEvent(target, event);
  }

  async function unplaced() {
    const drawn = await atlas('m/', {
      kind: 'AtlasCards',
      value: [
        card('a', true),
        card('b', false),
        card('c', false, { is_dir: true }),
      ],
    });
    const listed = () => [
      ...drawn.container.querySelectorAll('.atlas-unplaced .atlas-card'),
    ];
    const host = drawn.container.querySelector('.atlas-canvas') as Element;
    return { ...drawn, listed, host };
  }

  // 置いていないカードは、横のリストに出る。フォルダには印がつく。名前を押すと、そのページを開く。
  test('the unplaced cards are listed beside the canvas, and a click on a name opens its page', async () => {
    const { listed, wasm } = await unplaced();

    fireEvent.click(screen.getByText('C'), { ctrlKey: true });

    expect(listed().map(e => [e.className, e.textContent])).toEqual([
      ['atlas-card', 'B'],
      ['atlas-card folder', 'C'],
    ]);
    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: 'm/c', open_in: { kind: 'NewView' } },
    ]);
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

      expect(first).toEqual({ page_id: 'b', x: 0.25, y: 0.25 });
      expect(JSON.parse(moved.data.get(CARD_MIME) ?? '')).toEqual({
        page_id: 'b',
        x: 1,
        y: 0,
      });
      expect(moved.effectAllowed).toBe('move');
    }));

  // リストのカードをキャンバスの上で引くと、キャンバスがグループを示す。CtrlかCommandを押すと、入るグループも示す。Ctrlなしで落とすと、カードの上でもグループには入らず、その場所と、つかんだ場所をコアに渡す。
  test('a listed card over the canvas shows its group there, and a drop without Ctrl gives only the place to the core', async () => {
    const { host, canvas, wasm } = await unplaced();
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ page_id: 'b', x: 0.5, y: 0.25 }));
    canvas().target = 'a';

    drag('dragOver', host, { x: 140, y: 60 }, moved);
    const over = drag('dragOver', host, { x: 150, y: 70, meta: true }, moved);
    const dropped = drag('drop', host, { x: 150, y: 70 }, moved);

    expect([over, dropped, moved.dropEffect]).toEqual([false, false, 'move']);
    expect(canvas().given.slice(-3)).toEqual([
      ['over', [140, 60, false]],
      ['over', [150, 70, true]],
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
      },
    ]);
  });

  // Ctrlを押して落とすと、落とした先のグループにも入る。
  test('a drop with Ctrl also joins the group it lands on', async () => {
    const { host, canvas, wasm } = await unplaced();
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ page_id: 'b', x: 0, y: 0 }));
    canvas().target = 'a';

    drag('drop', host, { x: 100, y: 50, ctrl: true }, moved);

    expect(wasm.events).toEqual([
      { kind: 'DropAtlasCard', card_id: 'b', x: 0, y: 0, grab_x: 0, grab_y: 0 },
      { kind: 'GroupAtlasCards', card_ids: ['b'], target_id: 'a' },
    ]);
  });

  // ほかから引いてきた文字は、グループを示さず、落としても何もしない。キャンバスの準備がなければ、落としても何もしない。
  test('text from elsewhere shows no group and drops nothing, as does a canvas not yet ready', async () => {
    const { host, canvas, wasm } = await unplaced();
    const given = canvas().given.length;
    const text = dataTransfer(['text/plain']);
    drag('dragOver', host, { x: 0, y: 0 }, text);
    drag('drop', host, { x: 0, y: 0 }, text);
    const moved = dataTransfer([CARD_MIME]);
    moved.setData(CARD_MIME, JSON.stringify({ page_id: 'b', x: 0, y: 0 }));
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
