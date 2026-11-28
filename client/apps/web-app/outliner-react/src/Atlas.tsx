import { useEffect, useRef, useState, type DragEvent } from 'react';
import {
  AtlasPlace,
  eventLinkClicked,
  type AtlasCard,
  type Callout,
  type LinkCard,
  richLinkFile,
  richLinkPage,
  richLinkWeb,
  type OpenIn,
  type RichLink,
  type UnplacedCard,
} from './border/peta-types';
import { ATLAS_OPTIONS, loadAtlasScripts } from './atlasScripts';
import { AtlasDialog, type LinkSuggestions } from './atlas-dialog';
import { useCore, usePanel } from './context';
import { highlightCode, sizeImages } from './drawnBody';
import { checkWikiLink } from './WikiSuggestions';
import { openInOf } from './keys';

/** The canvas of atlas.js, as this shell uses it. */
interface AtlasCanvas {
  /** placesJson is where each was let go on the canvas: [{ id, x, y }] */
  setOnGroup(
    onGroup: (idsJson: string, targetId: string, placesJson: string) => void,
  ): void;
  setOnUngroup(onUngroup: (idsJson: string, placesJson: string) => void): void;
  setOnMove(
    onMove: (idsJson: string, parentId: string, placesJson: string) => void,
  ): void;
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
  /**
   * Add Card, the Edit of a card and Delete Card. bodyOf is the body of the item of a card or a
   * group as Markdown, which its Edit shows. A page card is named by its page, any other card
   * and a group by its item.
   */
  setOnCard(
    onAdd: (x: number, y: number, text: string) => void,
    onEdit: (cardId: string, text: string) => void,
    onRemove: (itemId: string) => void,
    bodyOf: (id: string) => string | undefined,
  ): void;
  /** The dialog of a text that the canvas asks this shell to open; done is called on OK */
  setOnText(
    onText: (
      asked: TextAsked,
      done: (side: string | null, text: string) => void,
    ) => void,
  ): void;
  /** link is a link of the first line of a card, as the core names the links of a page */
  setOnFollow(
    onFollow: (
      link: { kind: string; url: string },
      newTab: boolean,
      newWindow: boolean,
    ) => void,
  ): void;
  /** Remove from Atlas of a card: a page card named by its page, any other by its item */
  setOnRemove(onRemove: (cardId: string) => void): void;
  setCards(json: string): void;
  setCallouts(json: string): void;
  /** The point of the cards under a point of the screen; null before PixiJS has started. */
  toWorldX(clientX: number): number | null;
  toWorldY(clientY: number): number | null;
  dragOverFromList(
    clientX: number,
    clientY: number,
    ctrl: boolean,
    shift: boolean,
  ): void;
  endDragFromList(): void;
  groupTargetFromList(
    clientX: number,
    clientY: number,
    ctrl: boolean,
  ): string | null;
  /** The item of the innermost blob under a point, unless a card is there; null without Ctrl. */
  parentFromList(
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
      /** No longer called: every menu is drawn on the canvas */
      onMenu: () => void,
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
 * The drag data of a card of the lists beside the canvas. A type of its own keeps text
 * dragged from elsewhere from being taken as a card.
 */
const CARD_MIME = 'application/x-peta-atlas-card';

/**
 * A card of the lists being dragged: the id the core gave it and where on the card it was
 * grabbed, as a share of the card's width and height. A share and not pixels, as a long label
 * makes the card in the list taller than on the canvas.
 */
interface Grab {
  readonly card_id: string;
  readonly x: number;
  readonly y: number;
}

const NONE: readonly never[] = [];

/**
 * What the canvas of atlas.js asks the dialog of a text to show: the Edit of a card, a group
 * or a callout, Add Card, Add Callout, or View Text. id names the card of View Text. side is
 * null for a card, which has none.
 */
interface TextAsked {
  readonly name: string;
  readonly id: string | null;
  readonly text: string;
  readonly side: string | null;
  readonly mode: 'edit' | 'view';
}

/** A dialog of a text that the canvas asked for, open until it is closed */
interface Asking {
  readonly asked: TextAsked;
  readonly done: (side: string | null, text: string) => void;
  readonly count: number;
}

/** The panels of the atlas as they are now, which the Edit of a card reads its body from */
interface Panels {
  readonly cards: readonly AtlasCard[];
  readonly links: readonly LinkCard[];
  readonly callouts: readonly Callout[];
}

/**
 * The canvas of an atlas page, ported from the Leptos shell: the pages and folders of its
 * directory as cards, drawn by PixiJS in atlas.js. What is not on the canvas waits beside it
 * in two groups (ADR 053): the pages that no card links to, and the items of the atlas page
 * without a place. Either is placed by dragging it onto the canvas, at the top of the outline.
 * With Ctrl held, it also joins the group of the card it is dropped on, or goes under the item
 * of the blob it is dropped on. With Shift and Ctrl held, it goes under the card it is dropped
 * on.
 */
export default function Atlas({ dirPath }: { readonly dirPath: string }) {
  const core = useCore();
  const cards = usePanel('AtlasCards') ?? NONE;
  const links = usePanel('AtlasLinks') ?? NONE;
  const waiting = usePanel('AtlasUnplaced') ?? NONE;
  const callouts = usePanel('AtlasCallouts') ?? NONE;
  const host = useRef<HTMLDivElement>(null);
  const [canvas, setCanvas] = useState<AtlasCanvas | null>(null);
  const [asking, setAsking] = useState<Asking | null>(null);
  // The Edit opened later shows the body as it is then
  const panels = useRef<Panels>({ cards, links, callouts });
  useEffect(() => {
    panels.current = { cards, links, callouts };
  }, [cards, links, callouts]);

  // The canvas keeps the view of the folder it was made for
  useEffect(() => {
    let made: AtlasCanvas | undefined;
    let gone = false;
    loadAtlasScripts()
      .then(() => {
        if (gone) {
          return;
        }
        made = makeCanvas(host.current as HTMLDivElement, dirPath, core, id =>
          bodyIn(panels.current, id),
        );
        // Each dialog starts from what it is asked, also one opened in place of another
        made.setOnText((asked, done) =>
          setAsking(last => ({ asked, done, count: (last?.count ?? 0) + 1 })),
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
    // With Ctrl, what is dropped becomes a sibling of what it is dropped on: of a card, or of
    // the cards of a blob, whose parent it goes under. With Shift too, it goes under the card
    // it is dropped on, and nothing more happens on a blob (ADR 053). The core does the drop
    // and this in one change.
    const ctrl = event.ctrlKey || event.metaKey;
    const child = ctrl && event.shiftKey;
    canvas.endDragFromList();
    const target = canvas.groupTargetFromList(
      event.clientX,
      event.clientY,
      ctrl,
    );
    const parent = canvas.parentFromList(event.clientX, event.clientY, ctrl);
    const under = child ? target : parent;
    core.send({
      kind: 'DropAtlasCard',
      card_id: grab.card_id,
      x,
      y,
      grab_x: grab.x,
      grab_y: grab.y,
      target_id: child ? null : target,
      parent_id: target !== null && !child ? null : under,
    });
  };
  const draggedOver = (event: DragEvent) => {
    event.preventDefault();
    // Ctrl would make the drop a copy, and Shift with Ctrl a link, which the card does not
    // allow
    event.dataTransfer.dropEffect = 'move';
    // Text dragged from elsewhere joins no group
    if (!event.dataTransfer.types.includes(CARD_MIME)) {
      return;
    }
    canvas?.dragOverFromList(
      event.clientX,
      event.clientY,
      event.ctrlKey || event.metaKey,
      event.shiftKey,
    );
  };

  return (
    <div className="atlas">
      {/* The whole list scrolls, however many wait */}
      <div className="atlas-unplaced">
        <UnplacedGroup title="Pages" cards={waiting.filter(c => !c.added)} />
        <hr className="atlas-unplaced-separator" />
        <UnplacedGroup
          title="Not on Atlas"
          cards={waiting.filter(c => c.added)}
        />
      </div>
      {/* The browser's own menu would cover the menu of the canvas */}
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
      {asking !== null && (
        <TextDialog
          key={asking.count}
          asking={asking}
          close={() => setAsking(null)}
        />
      )}
    </div>
  );
}

/** The canvas of the folder `dir`, at the view kept for it, telling the core what is done. */
function makeCanvas(
  host: HTMLElement,
  dir: string,
  core: ReturnType<typeof useCore>,
  bodyOf: (id: string) => string | undefined,
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
    // A dragged group is placed as a card is, by the id of its item (ADR 053)
    (card_id, x, y) => core.send({ kind: 'PlaceAtlasCard', card_id, x, y }),
    () => {},
    (x, y, zoom) => saveView(dir, { x, y, zoom }),
    start.x,
    start.y,
    start.zoom,
    ATLAS_OPTIONS,
  );
  made.setOnGroup((idsJson, target_id, placesJson) =>
    core.send({
      kind: 'GroupAtlasCards',
      card_ids: JSON.parse(idsJson) as string[],
      target_id,
      places: placesOf(placesJson),
    }),
  );
  made.setOnUngroup((idsJson, placesJson) =>
    core.send({
      kind: 'UngroupAtlasCards',
      card_ids: JSON.parse(idsJson) as string[],
      places: placesOf(placesJson),
    }),
  );
  made.setOnMove((idsJson, parent_id, placesJson) =>
    core.send({
      kind: 'MoveAtlasCards',
      card_ids: JSON.parse(idsJson) as string[],
      parent_id,
      places: placesOf(placesJson),
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
  made.setOnCard(
    (x, y, text) => core.send({ kind: 'AddAtlasCard', x, y, text }),
    (card_id, text) => core.send({ kind: 'SetAtlasCardText', card_id, text }),
    item_id => core.send({ kind: 'RemoveAtlasLink', item_id }),
    bodyOf,
  );
  // The card stays as an item, and waits to be placed again
  made.setOnRemove(card_id =>
    core.send({ kind: 'RemoveAtlasCard', page_id: card_id }),
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

/**
 * The body of the item of a card, a callout or a group, as Markdown: a page card is named by its
 * page, any other by its item. A group is the item that cards or callouts are under.
 */
function bodyIn(panels: Panels, id: string): string | undefined {
  const { cards, links, callouts } = panels;
  const group = [...cards, ...links, ...callouts]
    .flatMap(c => c.groups)
    .find(g => g.id === id);
  return (
    cards.find(c => c.page_id === id)?.text ??
    links.find(l => l.item_id === id)?.text ??
    callouts.find(c => c.key === id)?.text ??
    group?.body
  );
}

/** The name the dialog of a text goes by in the wiki link queries, which no item has */
const DIALOG = 'atlas-dialog';

/** What the Preview leaves to the shell, as a page does with a body */
function finishBody(content: HTMLElement): void {
  sizeImages(content);
  highlightCode(content);
}

/**
 * The dialog of a text, with the Preview drawn by the core as a body of a page, and the page
 * names of a `[[` offered by the core, as for the editor of a page.
 */
function TextDialog({
  asking,
  close,
}: {
  readonly asking: Asking;
  readonly close: () => void;
}) {
  const core = useCore();
  const tagsDir = usePanel('Config')?.tags_dir ?? '';
  const wiki = usePanel('Outline')?.wiki;
  const { asked, done } = asking;
  const links: LinkSuggestions = {
    shown:
      wiki?.item_id === DIALOG
        ? {
            titles: wiki.suggestions,
            selected: Number(wiki.selected_index),
            x: wiki.x,
            y: wiki.y,
          }
        : null,
    typed: box => checkWikiLink(core, DIALOG, box),
    key: key => core.send({ kind: 'WikiKey', key }),
    picked: (text, cursor, title) => {
      const picked = core.wikiPicked(text, cursor, title);
      return picked === undefined
        ? undefined
        : (JSON.parse(picked) as { text: string; cursor: number });
    },
    closed: (before, text, cursor) => core.autoClosed(before, text, cursor),
    close: () => {
      if (core.current().panels.Outline?.wiki.item_id === DIALOG) {
        core.send({ kind: 'WikiQuery', item_id: DIALOG, context: null });
      }
    },
  };
  return (
    <AtlasDialog
      name={asked.name}
      mode={asked.mode}
      text={asked.text}
      side={asked.side}
      previewHtml={text => core.bodyHtml(text, tagsDir)}
      drawn={finishBody}
      links={asked.mode === 'edit' ? links : undefined}
      onOk={(side, text) => {
        close();
        done(side, text);
      }}
      onCancel={close}
    />
  );
}

/** Where the canvas says each dragged card was let go, as the core takes it. */
function placesOf(placesJson: string): AtlasPlace[] {
  const places = JSON.parse(placesJson) as AtlasPlace[];
  return places.map(place => new AtlasPlace(place.id, place.x, place.y));
}

/** A link of the text of a card as the core takes it. */
function richLinkOf(link: { kind: string; url: string }): RichLink {
  if (link.kind === 'Web') {
    return richLinkWeb(link.url);
  }
  return link.kind === 'File' ? richLinkFile(link.url) : richLinkPage(link.url);
}

/** A group of the list beside the canvas, under its title. */
function UnplacedGroup({
  title,
  cards,
}: {
  readonly title: string;
  readonly cards: readonly UnplacedCard[];
}) {
  return (
    <div className="atlas-unplaced-group">
      <div className="atlas-unplaced-title">{title}</div>
      {cards.map(c => (
        <Waiting key={c.id} card={c} />
      ))}
    </div>
  );
}

/** A card of the list beside the canvas: dragged onto the canvas, it is placed there. */
function Waiting({ card }: { readonly card: UnplacedCard }) {
  const core = useCore();
  const path = card.path;
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
          card_id: card.id,
          x: share(event.clientX, rect.left, rect.width),
          y: share(event.clientY, rect.top, rect.height),
        };
        event.dataTransfer.setData(CARD_MIME, JSON.stringify(grab));
        event.dataTransfer.effectAllowed = 'move';
      }}
    >
      {path === null ? (
        <span>{card.label}</span>
      ) : (
        <span
          className="atlas-card-label"
          onClick={event =>
            core.send({
              kind: 'GoToPage',
              path,
              open_in: openInOf(event.nativeEvent),
            })
          }
        >
          {card.label}
        </span>
      )}
    </div>
  );
}
