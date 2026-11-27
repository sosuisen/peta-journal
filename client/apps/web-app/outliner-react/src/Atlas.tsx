import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type DragEvent,
} from 'react';
import {
  eventLinkClicked,
  richLinkFile,
  richLinkPage,
  richLinkWeb,
  type AtlasCard,
  type OpenIn,
  type RichLink,
} from './border/peta-types';
import { ATLAS_OPTIONS, loadAtlasScripts } from './atlasScripts';
import { useCore, usePanel } from './context';
import { openInOf } from './keys';
import { useClosesOnOutside } from './menu';

/** The canvas of atlas.js, as this shell uses it. */
interface AtlasCanvas {
  setOnGroup(onGroup: (idsJson: string, targetId: string) => void): void;
  setOnUngroup(onUngroup: (idsJson: string) => void): void;
  setOnScale(onScale: (cardId: string, scale: number) => void): void;
  setOnGroupTitle(
    onGroupTitle: (groupId: string, side: string, title: string) => void,
  ): void;
  setOnAddCallout(
    onAddCallout: (x: number, y: number, side: string, text: string) => void,
  ): void;
  setOnCallout(
    onCallout: (key: string, side: string, text: string) => void,
  ): void;
  setOnMoveCallout(
    onMoveCallout: (key: string, x: number, y: number) => void,
  ): void;
  setOnScaleCallout(onScaleCallout: (key: string, scale: number) => void): void;
  setOnLink(
    onAdd: (x: number, y: number, target: string) => void,
    onSet: (itemId: string, target: string) => void,
    onRemove: (itemId: string) => void,
    onQuery: (query: string) => void,
  ): void;
  /** link is a link of the first line of a card, as the core names the links of a page */
  setOnFollow(
    onFollow: (
      link: { kind: string; url: string },
      newTab: boolean,
      newWindow: boolean,
    ) => void,
  ): void;
  setCards(json: string): void;
  setCallouts(json: string): void;
  setLinkSuggestions(json: string): void;
  /** The point of the cards under a point of the screen; null before PixiJS has started. */
  toWorldX(clientX: number): number | null;
  toWorldY(clientY: number): number | null;
  dragOverFromList(clientX: number, clientY: number, ctrl: boolean): void;
  endDragFromList(): void;
  groupTargetFromList(
    clientX: number,
    clientY: number,
    ctrl: boolean,
  ): string | null;
  destroy(): void;
}

declare global {
  interface Window {
    AtlasCanvas: new (
      host: HTMLElement,
      onOpen: (path: string, newTab: boolean, newWindow: boolean) => void,
      onPlace: (cardId: string, x: number, y: number) => void,
      onMenu: (pageId: string, x: number, y: number) => void,
      onView: (x: number, y: number, zoom: number) => void,
      panX: number,
      panY: number,
      zoom: number,
      options: typeof ATLAS_OPTIONS,
    ) => AtlasCanvas;
  }
}

const ATLAS_VIEWS_KEY = 'atlas-views';

/**
 * Where the hand tool and the wheel left the view of an atlas: how far it is moved from the
 * center of the canvas, and how much it is scaled.
 */
interface View {
  readonly x: number;
  readonly y: number;
  readonly zoom: number;
}

/** The view of each atlas as the browser stored it, by the dir path of the atlas. */
function loadViews(): Record<string, View> {
  try {
    return JSON.parse(localStorage.getItem(ATLAS_VIEWS_KEY) ?? '{}') as Record<
      string,
      View
    >;
  } catch {
    return {};
  }
}

function saveView(dir: string, view: View): void {
  localStorage.setItem(
    ATLAS_VIEWS_KEY,
    JSON.stringify({ ...loadViews(), [dir]: view }),
  );
}

/**
 * The drag data of an unplaced card. A type of its own keeps text dragged from elsewhere
 * from being taken as a card.
 */
const CARD_MIME = 'application/x-peta-atlas-card';

/**
 * An unplaced card being dragged: its page and where on the card it was grabbed, as a share
 * of the card's width and height. A share and not pixels, as a long label makes the card in
 * the list taller than on the canvas.
 */
interface Grab {
  readonly page_id: string;
  readonly x: number;
  readonly y: number;
}

const NONE: readonly never[] = [];

/** The menu of a right-clicked card: its page and where it opens on the screen. */
interface Menu {
  readonly pageId: string;
  readonly x: number;
  readonly y: number;
}

/**
 * The canvas of an atlas page, ported from the Leptos shell: the pages and folders of its
 * directory as cards, drawn by PixiJS in atlas.js. A card without a place waits in the list
 * of unplaced cards beside the canvas, and is placed by dragging it onto the canvas. With
 * Ctrl held, it also joins the group of the card or the group area it is dropped on.
 */
export default function Atlas({ dirPath }: { readonly dirPath: string }) {
  const core = useCore();
  const cards = usePanel('AtlasCards') ?? NONE;
  const links = usePanel('AtlasLinks') ?? NONE;
  const callouts = usePanel('AtlasCallouts') ?? NONE;
  const suggestions = usePanel('AtlasLinkSuggestions') ?? NONE;
  const host = useRef<HTMLDivElement>(null);
  const [canvas, setCanvas] = useState<AtlasCanvas | null>(null);
  const [menu, setMenu] = useState<Menu | null>(null);
  // The same function on every drawing, so the menu adds its listeners only once
  const closeMenu = useCallback(() => setMenu(null), []);

  // The canvas keeps the view of the folder it was made for
  useEffect(() => {
    let made: AtlasCanvas | undefined;
    let gone = false;
    loadAtlasScripts()
      .then(() => {
        if (gone) {
          return;
        }
        made = makeCanvas(
          host.current as HTMLDivElement,
          dirPath,
          core,
          setMenu,
        );
        setCanvas(made);
      })
      .catch((error: unknown) => console.error(error));
    return () => {
      gone = true;
      made?.destroy();
    };
  }, [core, dirPath]);

  // Apart, so a change of a callout does not draw every card again
  useEffect(() => {
    canvas?.setCards(core.atlasCards(cards, links));
  }, [canvas, core, cards, links]);
  useEffect(() => {
    canvas?.setCallouts(core.atlasCallouts(callouts));
  }, [canvas, core, callouts]);
  useEffect(() => {
    canvas?.setLinkSuggestions(JSON.stringify(suggestions));
  }, [canvas, suggestions]);

  const dropped = (event: DragEvent) => {
    event.preventDefault();
    const data = event.dataTransfer.getData(CARD_MIME);
    if (data === '' || canvas === null) {
      return;
    }
    const grab = JSON.parse(data) as Grab;
    // Only the canvas knows how far the hand tool has moved the view
    const x = canvas.toWorldX(event.clientX);
    const y = canvas.toWorldY(event.clientY);
    if (x === null || y === null) {
      return;
    }
    // With Ctrl, a card let go on a card or in the area of a group joins that group
    const ctrl = event.ctrlKey || event.metaKey;
    canvas.endDragFromList();
    const target = canvas.groupTargetFromList(
      event.clientX,
      event.clientY,
      ctrl,
    );
    core.send({
      kind: 'DropAtlasCard',
      card_id: grab.page_id,
      x,
      y,
      grab_x: grab.x,
      grab_y: grab.y,
    });
    if (target !== null) {
      core.send({
        kind: 'GroupAtlasCards',
        card_ids: [grab.page_id],
        target_id: target,
      });
    }
  };
  const draggedOver = (event: DragEvent) => {
    event.preventDefault();
    // Ctrl would make the drop a copy, which the card does not allow
    event.dataTransfer.dropEffect = 'move';
    // Text dragged from elsewhere joins no group
    if (!event.dataTransfer.types.includes(CARD_MIME)) {
      return;
    }
    canvas?.dragOverFromList(
      event.clientX,
      event.clientY,
      event.ctrlKey || event.metaKey,
    );
  };

  return (
    <div className="atlas">
      <div className="atlas-unplaced">
        <div className="atlas-unplaced-title">Unplaced</div>
        {cards
          .filter(c => c.bounds === null)
          .map(c => (
            <UnplacedCard key={c.page_id} card={c} />
          ))}
      </div>
      {/* The browser's own menu would cover the menu of a card */}
      <div
        ref={host}
        className="atlas-canvas"
        onDragOver={draggedOver}
        // Also when the pointer only passes onto a button over the canvas; the next
        // dragover shows the group again
        onDragLeave={() => canvas?.endDragFromList()}
        onDrop={dropped}
        onContextMenu={event => event.preventDefault()}
      >
        {/* HTML over the canvas, not drawn by PixiJS: it keeps its corner whatever the
            canvas shows. It names the directory it goes to. The root has none above. */}
        {dirPath !== '' && (
          <button
            className="atlas-up-btn"
            title="Up"
            onClick={() => core.send({ kind: 'AtlasUp' })}
          >
            <svg
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polyline points="14 9 9 4 4 9" />
              <path d="M20 20h-7a4 4 0 0 1-4-4V4" />
            </svg>
            <span>{core.parentDirLabel(dirPath)}</span>
          </button>
        )}
      </div>
      {menu !== null && <CardMenu menu={menu} close={closeMenu} />}
    </div>
  );
}

/** The canvas of the folder `dir`, at the view kept for it, telling the core what is done. */
function makeCanvas(
  host: HTMLElement,
  dir: string,
  core: ReturnType<typeof useCore>,
  setMenu: (menu: Menu) => void,
): AtlasCanvas {
  const start = loadViews()[dir] ?? { x: 0, y: 0, zoom: 1 };
  const made = new window.AtlasCanvas(
    host,
    // atlas.js gives Ctrl or Command as newTab, and Shift as newWindow: a new view and the browser
    (path, newTab, newWindow) => {
      let open_in: OpenIn = { kind: 'Here' };
      if (newWindow) {
        open_in = { kind: 'Browser' };
      } else if (newTab) {
        open_in = { kind: 'NewView' };
      }
      core.send({ kind: 'GoToPage', path, open_in });
    },
    (card_id, x, y) => core.send({ kind: 'PlaceAtlasCard', card_id, x, y }),
    (pageId, x, y) => setMenu({ pageId, x, y }),
    (x, y, zoom) => saveView(dir, { x, y, zoom }),
    start.x,
    start.y,
    start.zoom,
    ATLAS_OPTIONS,
  );
  made.setOnGroup((idsJson, target_id) =>
    core.send({
      kind: 'GroupAtlasCards',
      card_ids: JSON.parse(idsJson) as string[],
      target_id,
    }),
  );
  made.setOnUngroup(idsJson =>
    core.send({
      kind: 'UngroupAtlasCards',
      card_ids: JSON.parse(idsJson) as string[],
    }),
  );
  made.setOnScale((card_id, scale) =>
    core.send({ kind: 'ScaleAtlasCard', card_id, scale }),
  );
  made.setOnGroupTitle((group_id, side, title) =>
    core.send({ kind: 'SetAtlasGroupTitle', group_id, side, title }),
  );
  made.setOnAddCallout((x, y, side, text) =>
    core.send({ kind: 'AddAtlasCallout', x, y, side, text }),
  );
  made.setOnCallout((key, side, text) =>
    core.send({ kind: 'SetAtlasCallout', key, side, text }),
  );
  made.setOnMoveCallout((key, x, y) =>
    core.send({ kind: 'MoveAtlasCallout', key, x, y }),
  );
  made.setOnScaleCallout((key, scale) =>
    core.send({ kind: 'ScaleAtlasCallout', key, scale }),
  );
  made.setOnLink(
    (x, y, target) => core.send({ kind: 'AddAtlasLink', x, y, target }),
    (item_id, target) => core.send({ kind: 'SetAtlasLink', item_id, target }),
    item_id => core.send({ kind: 'RemoveAtlasLink', item_id }),
    query => core.send({ kind: 'SearchAtlasLinks', query }),
  );
  // As a click on a link of a page: Ctrl or Command as ctrl, Shift as shift (ADR 083)
  made.setOnFollow((link, newTab, newWindow) =>
    core.send(
      eventLinkClicked(richLinkOf(link), {
        shift: newWindow,
        ctrl: newTab,
        cmd: false,
      }),
    ),
  );
  return made;
}

/** A link of the text of a card as the core takes it. */
function richLinkOf(link: { kind: string; url: string }): RichLink {
  if (link.kind === 'Web') {
    return richLinkWeb(link.url);
  }
  return link.kind === 'File' ? richLinkFile(link.url) : richLinkPage(link.url);
}

/** A card of the list of unplaced cards: dragged onto the canvas, it is placed there. */
function UnplacedCard({ card }: { readonly card: AtlasCard }) {
  const core = useCore();
  return (
    <div
      className={card.is_dir ? 'atlas-card folder' : 'atlas-card'}
      draggable
      onDragStart={event => {
        // The card, not the label inside it that may be under the pointer
        const rect = event.currentTarget.getBoundingClientRect();
        const share = (at: number, from: number, size: number) =>
          Math.min(Math.max((at - from) / size, 0), 1);
        const grab: Grab = {
          page_id: card.page_id,
          x: share(event.clientX, rect.left, rect.width),
          y: share(event.clientY, rect.top, rect.height),
        };
        event.dataTransfer.setData(CARD_MIME, JSON.stringify(grab));
        event.dataTransfer.effectAllowed = 'move';
      }}
    >
      <span
        className="atlas-card-label"
        onClick={event =>
          core.send({
            kind: 'GoToPage',
            path: card.path,
            open_in: openInOf(event.nativeEvent),
          })
        }
      >
        {card.label}
      </span>
    </div>
  );
}

/** The menu of a right-clicked card on the canvas. */
function CardMenu({
  menu,
  close,
}: {
  readonly menu: Menu;
  readonly close: () => void;
}) {
  const core = useCore();
  useClosesOnOutside('.atlas-menu', close);
  return (
    <div
      className="atlas-menu"
      style={{ left: `${menu.x}px`, top: `${menu.y}px` }}
    >
      <button
        className="atlas-menu-item"
        onClick={() => {
          core.send({ kind: 'RemoveAtlasCard', page_id: menu.pageId });
          close();
        }}
      >
        Remove from Atlas
      </button>
    </div>
  );
}
