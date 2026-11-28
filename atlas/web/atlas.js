// The canvas of an atlas page: the cards of the pages and folders of its directory, drawn
// with PixiJS. PixiJS is the global `PIXI` of pixi.min.js beside this file. The origin of the card
// coordinates is the center of the canvas, marked by a gray cross. It stays in the center
// when the host is resized, so the cards keep their places around it.
//
// The hand tool moves the view: a drag with the right button, or with the left one
// between the cards and the groups. It moves the container of the cards and the cross, not
// the cards. A drag with the left button on the area of a group moves the group with all that
// is in it.
// The canvas is a tree of frames, as the outline of the atlas page is a tree of items (ADR 053).
// A frame is a Container: the canvas has one, and each group has one inside the frame of the
// group it is under, placed where the item of the group lies. The cards, the callouts and the
// outline of a group lie in its frame, so each is placed from its group, as its `atlas_bounds`
// are stored, and a group that moves takes them along. A right
// click on a group opens its menu, whose dialog gives the group a title. A double click on
// the group opens that dialog too. A right click between the cards and the groups adds a
// callout there: a character with a speech bubble beside it. The same menu adds a link card,
// a card for any page, which the core keeps as a wiki link in the atlas page.
// A callout or a link card is selected, dragged and scaled as a card is. A callout is not put
// into a group or taken out of one on the canvas.
// A double click on a callout or a link card, or its menu, opens its dialog.
// The menus and the dialog are made with @pixi/ui, the global `PIXI.ui` of pixi-ui.js beside this file.
// A canvas that is not editable only shows the atlas, as a static site does: a drag with
// either button moves the view, the wheel scales it, and a click on a label opens its page.
// It needs no @pixi/ui. A card whose label is hidden (`masked`) shows a softened blob only,
// and opens nothing.
// The wheel scales that container around the point under the pointer. With Ctrl, it scales
// the selected cards instead, each around its own center.
//
// A plain script that sets the global `AtlasCanvas`, loaded by index.html. Not a module
// of wasm-bindgen: Trunk puts a hash of the content in the name of a script it loads, so a
// browser never keeps an old copy. A module keeps the same name when it changes.

const CROSS_SIZE = 8;
// A press that moves less than this is a click, not a drag
const DRAG_THRESHOLD = 3;
// The longest time between the two clicks of a double click, in ms, as Windows has it by default.
// Not the click count of PixiJS: it allows only 200ms, and only on the same child, so a click
// on the face and one on the bubble would not count.
const DOUBLE_CLICK_TIME = 500;
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 5;
// How much one pixel of wheel scrolling scales the view
const ZOOM_PER_PIXEL = 0.0015;
// A wheel that counts in lines or pages, not pixels, is taken as this many pixels a line
const LINE_HEIGHT = 16;
// The labels get sharper up to this zoom; beyond it the textures would grow too large
const MAX_TEXT_ZOOM = 3;
// How long the wheel must rest before the labels are drawn again for the new zoom, in ms
const SHARPEN_DELAY = 150;
// The color a card takes while cards held with Ctrl are over it, to join its group. Red, as a
// light blue was hard to see on the blue of the selection and the blobs.
const GROUP_TARGET_TINT = 0xef4444;
// How far the outline of a group keeps away from the cards in it
const GROUP_PADDING = 14;
// How much the background of a card hides what lies behind it, while it is a drop target
const CARD_ALPHA = 0.4;
// How strong the icon behind the label of a card is
const ICON_ALPHA = 0.5;
// The frame of a group while the cards dragged out of it with Ctrl are still in it
const GROUP_HOLD_COLOR = 0x1d4ed8;
// The space between the label of a card and its sides
const CARD_PADDING = 12;
// The narrowest and the widest a card grows to fit its label
const CARD_MIN_WIDTH = 40;
const CARD_MAX_WIDTH = 320;
// The smallest a card can be scaled to, as the core keeps it. A larger card has no limit.
const MIN_CARD_SCALE = 0.5;
// The width of the white edge around the characters of a group title
const TITLE_STROKE = 3;
// How much of the height of its characters a group title reaches into the area of the group
const TITLE_INSET = 1 / 3;
// How much a group title hides what lies behind it
const TITLE_ALPHA = 0.7;
// How many points of each curve of an outline the title is laid along
const CURVE_SAMPLES = 16;
// The sides a group title can lie along, as the core stores them, with their names in the dialog
const TITLE_SIDES = [
  ['top', 'Top'],
  ['bottom', 'Bottom'],
  ['left', 'Left'],
  ['right', 'Right'],
];
const UI_FONT_SIZE = 13;
// The size of the character of a callout on the canvas
const CALLOUT_SIZE = 45;
// How far from its center the character of a callout reaches. The face leaves a margin in its
// SVG, so the tail of the bubble points at the face, not at the edge of the SVG.
const CALLOUT_REACH = CALLOUT_SIZE * 0.4;
// The text of a callout, the space around it in its bubble, and the widest the text grows
const CALLOUT_FONT_SIZE = 11;
const BUBBLE_PADDING = 6;
const BUBBLE_TEXT_WIDTH = 144;
// How long the tail of a bubble is, and how wide where it leaves the bubble
const BUBBLE_TAIL = 9;
const BUBBLE_TAIL_BASE = 9;
// The space between the face and the tip of the tail of its bubble
const BUBBLE_GAP = 6;
// How far the line of a bubble wanders in and out, as if drawn by hand, and how far apart the
// points of that line are
const BUBBLE_WOBBLE = 1.2;
const BUBBLE_STEP = 8;
// The label of a link card, in the color of a link
const LINK_LABEL_COLOR = 0x1d4ed8;
// The page names offered under the input of the dialog of a link, and the height of each row
const LINK_ROWS = 8;
const LINK_ROW_HEIGHT = 26;
// The handwritten font of the group titles and the callouts, copied to the root of the site by
// index.html. From the root: a relative path would be read from the folder of the page.
const TITLE_FONT = { family: '851 Tegaki Zatsu', url: '/851tegaki_zatsu_normal_0883.ttf' };
// How much of the blob of a card with a hidden label shows. The scene gives it a white blob,
// which would glare at full strength on the paper of the canvas.
const MASKED_ALPHA = 0.7;

window.AtlasCanvas = class AtlasCanvas {
  // onOpen(path, newTab, newWindow) opens the page of a card whose label is clicked.
  // onPlace(id, x, y) places a card with its center at a point of the canvas, or a group, named
  // by the id of its item, with the place of its item there.
  // onMenu(pageId, clientX, clientY) opens the menu of a card that is right-clicked.
  // onView(panX, panY, zoom) keeps the view once the hand tool or the wheel has stopped.
  // panX, panY and zoom are the view to start with, as onView last gave them.
  // options: { editable, labelFont, titleFont }. editable is true unless it is false. A font is
  // { family, url }; labelFont is the font of the labels of the cards, and the cards are drawn
  // once it has come, so a label takes the width it has in that font. Without it, the labels
  // take the font of the page. titleFont is the font of the group titles and the callouts.
  constructor(host, onOpen, onPlace, onMenu, onView, panX, panY, zoom, options = {}) {
    this.onOpen = onOpen;
    this.onPlace = onPlace;
    this.onMenu = onMenu;
    this.onView = onView;
    this.drag = null;
    // How far the hand tool has moved the view from the center of the canvas
    this.pan = {
      x: Number.isFinite(panX) ? panX : 0,
      y: Number.isFinite(panY) ? panY : 0,
    };
    this.zoom = Number.isFinite(zoom) ? Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom)) : 1;
    this.panning = null;
    // The ids of the selected cards. Kept by id, as a new list of cards makes new views.
    this.selected = new Set();
    this.data = [];
    this.calloutList = [];
    this.destroyed = false;
    this.editable = options.editable !== false;
    this.labelFont = options.labelFont ?? null;
    this.titleFont = options.titleFont ?? TITLE_FONT;
    // The cards are drawn once this is set, after the font of the labels has come
    this.started = false;
    const labelFontReady = this.labelFont
      ? PIXI.Assets.load({ src: this.labelFont.url, data: { family: this.labelFont.family } }).catch(e =>
          console.warn('atlas: cannot load the font of the labels', e),
        )
      : Promise.resolve();
    this.app = new PIXI.Application();
    this.ready = this.app
      .init({
        resizeTo: host,
        // A pale ochre, as of old paper
        background: '#f8f2e4',
        antialias: true,
        resolution: window.devicePixelRatio || 1,
        autoDensity: true,
      })
      .then(() => {
        // The page may have been left while PixiJS was starting
        if (this.destroyed) {
          this.app.destroy({ removeView: true }, { children: true });
          return;
        }
        host.appendChild(this.app.canvas);
        this.world = new PIXI.Container();
        // The frame of the canvas, which what is in no group lies in, and the frames of the
        // groups by the ids of their items
        this.root = frame(null);
        this.frames = new Map();
        this.world.addChild(cross(), this.root);
        // The menu and the dialog, in pixels of the screen above the scaled view
        this.overlay = new PIXI.Container();
        this.app.stage.addChild(this.world, this.overlay);
        this.app.renderer.on('resize', () => this.center());
        // resizeTo follows the window only. The host also changes size when the list of
        // unplaced cards beside it fills or empties, which would leave the center off.
        this.resizeObserver = new ResizeObserver(() => this.app.resize());
        this.resizeObserver.observe(host);
        this.listenForHand();
        this.center();
        return labelFontReady;
      })
      .then(() => {
        if (this.destroyed || !this.root) {
          return;
        }
        this.started = true;
        this.build();
        this.loadTitleFont();
      });
  }

  // The stage takes a press anywhere on the canvas, also between the cards
  listenForHand() {
    const stage = this.app.stage;
    stage.eventMode = 'static';
    stage.hitArea = this.app.screen;
    stage.on('pointerdown', e => this.startPan(e));
    stage.on('globalpointermove', e => this.movePan(e));
    stage.on('pointerup', e => this.endPan(e));
    stage.on('pointerupoutside', e => this.endPan(e));
    // A listener of its own, not the wheel event of PixiJS: that one is passive, so it
    // cannot keep the page from scrolling
    this.app.canvas.addEventListener('wheel', e => this.zoomAt(e), { passive: false });
    this.listenForPinch();
    // The right button moves the view; the menu of the browser would open at its end. An
    // editable canvas is in the app, which keeps that menu away itself.
    if (!this.editable) {
      this.app.canvas.addEventListener('contextmenu', e => e.preventDefault());
    }
  }

  // The fonts of the labels and of the titles, each followed by the font of the page for the
  // characters it does not have
  fonts() {
    const page = uiFont();
    return {
      label: this.labelFont ? `"${this.labelFont.family}", ${page}` : page,
      title: `"${this.titleFont.family}", ${page}`,
    };
  }

  // Two fingers scale the view, as the wheel does: the point between them stays under them,
  // and moving them together moves the view. The touches are read from the canvas itself, as
  // PixiJS gives each pointer alone. The browser would scale or scroll the page with the same
  // fingers, so it is told to leave the touches to the canvas.
  listenForPinch() {
    const canvas = this.app.canvas;
    canvas.style.touchAction = 'none';
    this.touches = new Map();
    this.pinch = null;
    const point = e => {
      const rect = canvas.getBoundingClientRect();
      return { x: e.clientX - rect.left, y: e.clientY - rect.top };
    };
    canvas.addEventListener('pointerdown', e => {
      if (e.pointerType !== 'touch') {
        return;
      }
      this.touches.set(e.pointerId, point(e));
      if (this.touches.size === 2) {
        this.startPinch();
      }
    });
    canvas.addEventListener('pointermove', e => {
      if (!this.touches.has(e.pointerId)) {
        return;
      }
      this.touches.set(e.pointerId, point(e));
      if (this.pinch && this.touches.size === 2) {
        this.movePinch();
      }
    });
    const end = e => {
      if (!this.touches.delete(e.pointerId)) {
        return;
      }
      if (this.pinch) {
        this.endPinch();
      }
    };
    canvas.addEventListener('pointerup', end);
    canvas.addEventListener('pointercancel', end);
  }

  // The second finger ends the pan of the first: the view follows the pair from here
  startPinch() {
    if (this.dialog) {
      return;
    }
    this.closeMenu();
    this.panning = null;
    this.showHand();
    const { center, distance } = this.pinchSpan();
    this.pinch = { center, distance, zoom: this.zoom };
  }

  movePinch() {
    const { center, distance } = this.pinchSpan();
    const pinch = this.pinch;
    this.pan = { x: this.pan.x + center.x - pinch.center.x, y: this.pan.y + center.y - pinch.center.y };
    pinch.center = center;
    this.zoomTo((this.zoom * distance) / pinch.distance, center.x, center.y);
    pinch.distance = distance;
  }

  // The finger that stays may go on alone; it starts no pan, as its press has passed
  endPinch() {
    this.pinch = null;
    this.keepView();
  }

  // The point between the two fingers, in pixels of the canvas, and how far apart they are
  pinchSpan() {
    const [a, b] = [...this.touches.values()];
    return {
      center: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
      distance: Math.max(1, Math.hypot(b.x - a.x, b.y - a.y)),
    };
  }

  // The point under the pointer stays where it is while the view is scaled
  zoomAt(e) {
    e.preventDefault();
    // The view stays as it is under the dialog; the menu would be left behind
    if (this.dialog) {
      return;
    }
    this.closeMenu();
    const delta = e.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? e.deltaY : e.deltaY * LINE_HEIGHT;
    if ((e.ctrlKey || e.metaKey) && this.selected.size > 0) {
      this.scaleSelected(Math.exp(-delta * ZOOM_PER_PIXEL));
      return;
    }
    const rect = this.app.canvas.getBoundingClientRect();
    this.zoomTo(this.zoom * Math.exp(-delta * ZOOM_PER_PIXEL), e.clientX - rect.left, e.clientY - rect.top);
  }

  // Scales the view to `zoom`, within its limits, so that the point (px, py) of the canvas
  // stays where it is
  zoomTo(wanted, px, py) {
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, wanted));
    const ratio = zoom / this.zoom;
    const cx = this.app.screen.width / 2;
    const cy = this.app.screen.height / 2;
    // The origin sits at the center plus the pan; it moves toward or away from the pointer
    this.pan = {
      x: px - (px - cx - this.pan.x) * ratio - cx,
      y: py - (py - cy - this.pan.y) * ratio - cy,
    };
    this.zoom = zoom;
    this.center();
    // Drawing every label again on each step of the wheel or the fingers would make the zoom
    // stutter. The view is kept then too, not on each step.
    clearTimeout(this.sharpenTimer);
    this.sharpenTimer = setTimeout(() => {
      this.sharpenText();
      this.keepView();
    }, SHARPEN_DELAY);
  }

  // The selected cards grow or shrink by the factor, each around its center, as the pivot is
  // there. Only the outlines of their groups are drawn again. The scales are saved once the
  // wheel rests, as each save gives a new list of cards.
  scaleSelected(factor) {
    const views = this.cardViews().filter(v => this.selected.has(v.cardData.id));
    for (const view of views) {
      const scale = Math.max(MIN_CARD_SCALE, view.scale.x * factor);
      view.scale.set(scale);
      view.cardData.width = view.cardWidth * scale;
      view.cardData.height = view.cardHeight * scale;
    }
    const extras = this.extras().filter(v => this.selected.has(v.extraId));
    for (const view of extras) {
      view.scale.set(Math.max(MIN_CARD_SCALE, view.scale.x * factor));
    }
    // Not a drag: the outlines of the groups of these cards and callouts only, with no frame shown
    const still = list => list.map(v => ({ view: v, x: v.x, y: v.y }));
    this.drawGroups({ origins: still(views), extras: still(extras), moved: false });
    clearTimeout(this.scaleTimer);
    this.scaleTimer = setTimeout(() => {
      this.sharpenText();
      if (this.onScale) {
        for (const view of views) {
          this.onScale(view.cardData.id, view.scale.x);
        }
      }
      for (const view of extras) {
        view.saveScale(view.scale.x);
      }
    }, SHARPEN_DELAY);
  }

  // The labels are drawn in as many pixels as they cover on the screen, so they stay sharp
  // when the view or the card is scaled up. Not ahead of time for the largest zoom: every
  // label would then hold a texture many times its size.
  textResolution(scale = 1) {
    const zoom = Math.min(MAX_TEXT_ZOOM, Math.max(1, this.zoom * scale));
    return (window.devicePixelRatio || 1) * zoom;
  }

  // The icons are drawn once for the largest zoom of the labels: they are small, and a texture
  // is shared by every card of the same tags
  iconResolution() {
    return (window.devicePixelRatio || 1) * MAX_TEXT_ZOOM;
  }

  sharpenText() {
    for (const view of this.cardViews()) {
      const resolution = this.textResolution(view.scale.x);
      // The label, or each part of a row of them
      for (const text of view.cardLabel.anchor ? [view.cardLabel] : view.cardLabel.children) {
        if (text.resolution !== resolution) {
          text.resolution = resolution;
        }
      }
    }
    for (const view of this.extras()) {
      const resolution = this.textResolution(view.scale.x);
      if (view.extraLabel.resolution !== resolution) {
        view.extraLabel.resolution = resolution;
      }
    }
    const resolution = this.textResolution();
    for (const area of this.areas()) {
      for (const char of area.titleChars?.children ?? []) {
        if (char.resolution !== resolution) {
          char.resolution = resolution;
        }
      }
    }
  }

  startPan(e) {
    // The menu and the dialog take their own presses. A press anywhere else closes the menu.
    if (this.overlay.children.length > 0 && within(e.target, this.overlay)) {
      return;
    }
    this.closeMenu();
    const card = cardOf(e.target);
    if (!this.editable) {
      this.startViewPan(e, card);
      return;
    }
    // The left button on a card drags the card, on a group its cards; elsewhere it moves the
    // view
    if (e.button === 0 && extraOf(e.target)) {
      return;
    }
    if (e.button === 0 && !card && e.target?.groupArea && !this.panning && !this.groupDrag) {
      this.startGroupDrag(e, e.target);
      return;
    }
    const hand = e.button === 2 || (e.button === 0 && !card);
    if (!hand || this.panning) {
      return;
    }
    this.panning = {
      button: e.button,
      start: { x: e.global.x, y: e.global.y },
      origin: { x: this.pan.x, y: this.pan.y },
      moved: false,
      // A right click that does not move opens the menu of this card, of this group, of this
      // callout or link card, or else of the canvas at this point
      card,
      area: e.target?.groupArea ? e.target : null,
      extra: extraOf(e.target),
      point: this.world.toLocal(e.global),
      titlePath: titlePathOf(e.target),
    };
    this.showHand();
  }

  // A press anywhere on a canvas that is not editable moves the view. One that does not move is
  // a click, which opens the page of a label.
  startViewPan(e, card) {
    if (this.panning || (e.button !== 0 && e.button !== 2)) {
      return;
    }
    this.panning = {
      button: e.button,
      start: { x: e.global.x, y: e.global.y },
      origin: { x: this.pan.x, y: this.pan.y },
      moved: false,
      card,
      onLabel: Boolean(card) && e.target === card.cardLabel,
      runLink: runLinkOf(e.target),
      titlePath: titlePathOf(e.target),
    };
    this.showHand();
  }

  movePan(e) {
    if (this.groupDrag) {
      this.moveGroupDrag(e);
      return;
    }
    const panning = this.panning;
    if (!panning || this.pinch) {
      return;
    }
    const dx = e.global.x - panning.start.x;
    const dy = e.global.y - panning.start.y;
    if (!panning.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) {
      return;
    }
    if (!panning.moved) {
      panning.moved = true;
      this.showHand();
    }
    this.pan = { x: panning.origin.x + dx, y: panning.origin.y + dy };
    this.center();
  }

  endPan(e) {
    if (this.groupDrag && e.button === 0) {
      this.endGroupDrag(e);
      return;
    }
    const panning = this.panning;
    if (!panning || e.button !== panning.button) {
      return;
    }
    this.panning = null;
    this.showHand();
    if (panning.moved) {
      this.keepView();
    } else if (panning.button === 0 && panning.titlePath) {
      this.onOpen(panning.titlePath, e.ctrlKey || e.metaKey, e.shiftKey);
    } else if (!this.editable) {
      const data = panning.card?.cardData;
      if (panning.button === 0 && panning.runLink && !data?.masked) {
        this.onFollow?.(panning.runLink, e.ctrlKey || e.metaKey, e.shiftKey);
      } else if (panning.button === 0 && panning.onLabel && opens(data)) {
        this.onOpen(data.path, e.ctrlKey || e.metaKey, e.shiftKey);
      } else if (panning.button === 2 && data && !data.masked) {
        // A page that cannot be changed still shows the whole text of its cards
        this.openMenu([['Show Text', () => this.onShowText?.(data.id, e.clientX, e.clientY)]], e.global.x, e.global.y);
      }
    } else if (panning.button === 2 && panning.card?.linkData && !panning.card.destroyed) {
      const data = panning.card.linkData;
      const showText = ['Show Text', () => this.onShowText?.(data.id, e.clientX, e.clientY)];
      // A plain card links to no page, so it has no link to edit
      const items = data.isPlain
        ? [showText]
        : [['Edit Link', () => this.openLinkDialog(data)], ['Remove Link', () => this.onRemoveLink?.(data.id)], showText];
      this.openMenu(items, e.global.x, e.global.y);
    } else if (panning.button === 2 && panning.card) {
      this.onMenu(panning.card.cardData.id, e.clientX, e.clientY);
    } else if (panning.button === 2 && panning.extra?.calloutData && !panning.extra.destroyed) {
      const data = panning.extra.calloutData;
      this.openMenu([['Edit Callout', () => this.openCalloutDialog(data)]], e.global.x, e.global.y);
    } else if (panning.button === 2 && panning.area && !panning.area.destroyed) {
      const group = panning.area.label;
      this.openMenu([['Edit Title', () => this.openTitleDialog(group)]], e.global.x, e.global.y);
    } else if (panning.button === 2) {
      const { x, y } = panning.point;
      this.openMenu(
        [
          ['Add Link', () => this.openLinkDialog({ x, y })],
          ['Add Callout', () => this.openCalloutDialog({ x, y })],
        ],
        e.global.x,
        e.global.y,
      );
    } else if (panning.button === 0 && !panning.card) {
      // A click between the cards lets go of the selection
      this.select([]);
    }
  }

  // The font file is large, so the titles and the callouts show in the font of the app until it
  // has come; then they are drawn again in it. PixiJS draws a Text once and would keep the first font.
  loadTitleFont() {
    PIXI.Assets.load({ src: this.titleFont.url, data: { family: this.titleFont.family } })
      .then(() => {
        if (!this.destroyed && !this.drag && !this.groupDrag) {
          this.build();
        }
      })
      .catch(e => console.warn('atlas: cannot load the font of the group titles', e));
  }

  // A menu at a point of the screen, for a group, a callout, a link card or the canvas. Its items
  // are [name, action] pairs.
  openMenu(items, x, y) {
    this.closeMenu();
    const height = 8 + items.length * 30;
    const menu = new PIXI.Container();
    menu.addChild(
      new PIXI.Graphics()
        .roundRect(0, 0, 140, height, 6)
        .fill(0xffffff)
        .stroke({ color: 0xdddddd, width: 1 }),
    );
    items.forEach(([name, action], i) => {
      const item = new PIXI.ui.FancyButton({
        defaultView: new PIXI.Graphics().rect(0, 0, 140, 30).fill(0xffffff),
        hoverView: new PIXI.Graphics().rect(0, 0, 140, 30).fill(0xf0f0f0),
        text: uiText(name),
        // The text at the left, as in the other menus of the app
        defaultTextAnchor: { x: 0, y: 0.5 },
        textOffset: { x: -70 + 12, y: 0 },
      });
      item.onPress.connect(() => {
        this.closeMenu();
        action();
      });
      // A button lies with its top left at its position
      item.position.set(0, 4 + i * 30);
      menu.addChild(item);
    });
    // Kept on the screen near its right and bottom edges
    menu.position.set(
      Math.min(x, this.app.screen.width - 144),
      Math.min(y, this.app.screen.height - height - 4),
    );
    this.menu = menu;
    this.overlay.addChild(menu);
    this.menuKeys = e => {
      if (e.key === 'Escape') {
        this.closeMenu();
      }
    };
    window.addEventListener('keydown', this.menuKeys);
  }

  closeMenu() {
    if (!this.menu) {
      return;
    }
    window.removeEventListener('keydown', this.menuKeys);
    this.menu.destroy({ children: true });
    this.menu = null;
  }

  // The dialog of the title of a group: the title and the side of the outline it lies along
  openTitleDialog(group) {
    const data = this.groupData(group);
    this.openTextDialog({
      name: 'Edit Title',
      placeholder: 'Title',
      text: data?.title,
      side: data?.titleSide,
      done: (side, text) => this.onGroupTitle?.(group, side, text),
    });
  }

  // A group as the cards in it carry it
  groupData(group) {
    for (const data of this.data) {
      const found = groupsOf(data).find(g => g.id === group);
      if (found) {
        return found;
      }
    }
    return null;
  }

  // The dialog of a callout: its text and the side of the character its bubble is on. A
  // callout without a key is a new one at its point.
  openCalloutDialog(callout) {
    this.openTextDialog({
      name: callout.key ? 'Edit Callout' : 'Add Callout',
      placeholder: 'Text',
      text: callout.text,
      side: callout.side,
      done: (side, text) => {
        if (callout.key) {
          this.onCallout?.(callout.key, side, text);
        } else {
          this.onAddCallout?.(callout.x, callout.y, side, text);
        }
      },
    });
  }

  // The dialog of a link card, or of a new one at a point: the page it links to, with the page
  // names that match what is typed offered under it. A click on a name takes it at once.
  openLinkDialog(link) {
    this.openTextDialog({
      name: link.id ? 'Edit Link' : 'Add Link',
      placeholder: 'Page',
      text: link.path,
      suggest: true,
      done: (side, target) => {
        if (link.id) {
          this.onSetLink?.(link.id, target);
        } else {
          this.onAddLink?.(link.x, link.y, target);
        }
      },
    });
  }

  // A dialog of a text, with a side to pick or with page names to pick from. OK or Enter gives
  // them to `done(side, text)`; the core takes an empty text as none. Cancel, Escape and a press
  // beside the dialog leave things as they were.
  openTextDialog({ name, placeholder, text, side, suggest = false, done }) {
    const sideIndex = Math.max(
      0,
      TITLE_SIDES.findIndex(([s]) => s === side),
    );
    const width = 340;
    const height = suggest ? 200 + LINK_ROWS * LINK_ROW_HEIGHT - 40 : 200;
    const input = new PIXI.ui.Input({
      bg: new PIXI.Graphics()
        .roundRect(0, 0, width - 60, 32, 4)
        .fill(0xffffff)
        .stroke({ color: 0xc0c4cc, width: 1 }),
      placeholder,
      value: text ?? '',
      padding: { left: 8, right: 8 },
      // A long text is cut off inside the padding, and scrolls with the cursor while edited.
      // Without the mask, it runs past the edge of the input.
      addMask: true,
      textStyle: { fontFamily: uiFont(), fontSize: UI_FONT_SIZE, fill: 0x222222 },
    });
    const sides = new PIXI.ui.RadioGroup({
      items: TITLE_SIDES.map(([, name]) => new PIXI.ui.CheckBox({
        text: name,
        style: {
          unchecked: new PIXI.Graphics().circle(8, 8, 7).fill(0xffffff).stroke({ color: 0x9aa0a6, width: 1.5 }),
          checked: new PIXI.Graphics()
            .circle(8, 8, 7).fill(0xffffff).stroke({ color: 0x1a73e8, width: 1.5 })
            .circle(8, 8, 4).fill(0x1a73e8),
          text: { fontFamily: uiFont(), fontSize: UI_FONT_SIZE, fill: 0x222222 },
          textOffset: { x: 4 },
        },
      })),
      type: 'horizontal',
      elementsMargin: 16,
      selectedItem: sideIndex,
    });
    sides.y = 48;
    const content = new PIXI.Container();
    content.addChild(input);
    // The page names, shown by the core through setLinkSuggestions as they are found
    const rows = new PIXI.Container();
    rows.y = 40;
    if (suggest) {
      content.addChild(rows);
      input.onChange.connect(value => this.onLinkQuery?.(value));
    } else {
      content.addChild(sides);
    }
    const dialog = new PIXI.ui.Dialog({
      background: new PIXI.Graphics()
        .roundRect(0, 0, width, height, 10)
        .fill(0xffffff)
        .stroke({ color: 0xdddddd, width: 1 }),
      backdropColor: 0x000000,
      backdropAlpha: 0.15,
      title: uiText(name, 15),
      content,
      width,
      height,
      padding: 20,
      buttons: [dialogButton('Cancel', false), dialogButton('OK', true)],
      closeOnBackdropClick: true,
    });
    dialog.position.set(this.app.screen.width / 2, this.app.screen.height / 2);
    // The Dialog puts its content in a ScrollBox. That box takes a drag once its content is even
    // a little larger than it, as the edge of the input is, and the drag shifts the content
    // around. The content fits, so the box is kept from taking a press; its items still take
    // theirs. @pixi/ui has no option for this, and the box turns itself on again when it lays
    // out its items, so that switch is turned off too.
    dialog.scrollBox.setInteractive = () => {};
    dialog.scrollBox.eventMode = 'passive';
    const finish = ok => {
      if (this.dialog !== dialog) {
        return;
      }
      this.dialog = null;
      window.removeEventListener('keydown', keys, true);
      if (suggest) {
        this.showSuggestions = null;
        this.onLinkQuery?.('');
      }
      if (ok) {
        done(TITLE_SIDES[sides.selected][0], input.value);
      }
      dialog.close();
      // After the close has hidden it, which the next frame does
      setTimeout(() => {
        if (!dialog.destroyed) {
          dialog.destroy({ children: true });
        }
      }, 0);
    };
    this.closeDialog = () => finish(false);
    if (suggest) {
      this.showSuggestions = titles => {
        rows.removeChildren().forEach(r => r.destroy({ children: true }));
        titles.slice(0, LINK_ROWS).forEach((title, i) => {
          const row = suggestionRow(title, width - 60);
          row.onPress.connect(() => {
            input.value = title;
            finish(true);
          });
          row.y = i * LINK_ROW_HEIGHT;
          rows.addChild(row);
        });
      };
    }
    // Enter is taken only once the text is no longer being composed by an IME
    const keys = e => {
      if (e.key === 'Escape') {
        e.preventDefault();
        finish(false);
      } else if (e.key === 'Enter' && !e.isComposing && e.keyCode !== 229) {
        e.preventDefault();
        finish(true);
      }
    };
    dialog.onSelect.connect(index => finish(index === 1));
    dialog.onClose.connect(() => finish(false));
    window.addEventListener('keydown', keys, true);
    this.dialog = dialog;
    this.overlay.addChild(dialog);
    dialog.open();
    // After the click that opened the dialog: the input stops editing on a press elsewhere
    setTimeout(() => {
      if (this.dialog === dialog) {
        input._startEditing();
      }
    }, 0);
  }

  // A group is dragged by its frame, without Ctrl: all that is in it moves with it, and nothing
  // in it is placed anew. It keeps its cards and joins no other group.
  startGroupDrag(e, area) {
    const node = this.frames.get(area.label);
    this.groupDrag = {
      group: area.label,
      node,
      start: { x: e.global.x, y: e.global.y },
      origin: { x: node.x, y: node.y },
      // No card moves in its own frame
      origins: [],
      moved: false,
    };
    // Nothing else takes the press or shows its cursor while the group moves
    this.root.interactiveChildren = false;
  }

  moveGroupDrag(e) {
    const drag = this.groupDrag;
    const dx = e.global.x - drag.start.x;
    const dy = e.global.y - drag.start.y;
    if (!drag.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) {
      return;
    }
    if (!drag.moved) {
      drag.moved = true;
      this.showFrames(drag);
      this.app.stage.cursor = 'grabbing';
    }
    // The pointer moves in pixels of the screen, the frame in units of the scaled view
    drag.node.position.set(drag.origin.x + dx / this.zoom, drag.origin.y + dy / this.zoom);
    // Only the outlines of the groups around it change: its own moves with it
    this.drawGroups(drag);
  }

  endGroupDrag(e) {
    const drag = this.groupDrag;
    this.groupDrag = null;
    this.root.interactiveChildren = true;
    this.app.stage.cursor = 'default';
    if (!drag.moved) {
      // A click on a group, as between the cards, lets go of the selection. A double click
      // opens the dialog of its title.
      this.select([]);
      const last = this.lastGroupClick;
      if (last && last.group === drag.group && e.timeStamp - last.time < DOUBLE_CLICK_TIME) {
        this.lastGroupClick = null;
        this.openTitleDialog(drag.group);
      } else {
        this.lastGroupClick = { group: drag.group, time: e.timeStamp };
      }
      return;
    }
    this.showFrames(null);
    // The group is placed as a card is, by the id of its item: only its own place changes
    const { x, y } = onCanvas(drag.node.outer, drag.node.x, drag.node.y);
    this.onPlace(drag.group, x, y);
  }

  // The views of the cards, in the order they are drawn: those of a group before those of the
  // group it is in, and the later of two after the earlier
  cardViews() {
    const out = [];
    const walk = node => {
      node.inner.children.forEach(walk);
      out.push(...node.cardLayer.children);
    };
    walk(this.root);
    return out;
  }

  // The areas of the groups, in the order they are drawn: that of a group before those of the
  // groups in it
  areas() {
    return framesIn(this.root)
      .map(node => node.area)
      .filter(Boolean);
  }

  // The frame of the innermost group of a chain of groups, the outermost first; that of the
  // canvas for no group. A frame that is not there yet is made in the frame of the group above
  // it. Each lies where its group says, counted from the group above it.
  frameOf(chain) {
    let node = this.root;
    for (const group of chain) {
      let inner = this.frames.get(group.id);
      if (!inner) {
        inner = frame(node);
        inner.groupId = group.id;
        this.frames.set(group.id, inner);
        node.inner.addChild(inner);
      }
      // A scene made before the groups had places gives none: the group lies at the origin
      inner.position.set(group.x ?? 0, group.y ?? 0);
      node = inner;
    }
    return node;
  }

  // The ids of the groups whose outlines a drag changes: those of the dragged cards and
  // callouts, or those around a dragged group, and with `own` that group itself
  groupsOfDrag(drag, own = false) {
    if (drag.node) {
      const around = chainOf(drag.node.outer);
      return new Set(own ? [...around, drag.group] : around);
    }
    const moving = [...drag.origins, ...(drag.extras ?? [])];
    return new Set(moving.flatMap(o => groupsOf(dataOf(o.view)).map(g => g.id)));
  }

  select(ids) {
    this.selected = new Set(ids);
    this.showSelection();
  }

  showSelection() {
    for (const view of this.cardViews()) {
      view.cardSelection.visible = this.selected.has(view.cardData.id);
    }
    for (const view of this.extras()) {
      view.extraSelection.visible = this.selected.has(view.extraId);
    }
  }

  // The views that are no cards: the callouts. They are selected, dragged and scaled as the
  // cards are, but are not put into a group or taken out of one. Each knows its id, its label,
  // its selection ring, and how a move or a scale of it is saved.
  extras() {
    return framesIn(this.root).flatMap(node => node.calloutLayer.children);
  }

  // A card or a callout taken off the canvas is no longer selected
  keepSelection() {
    const ids = new Set([...this.data.map(d => d.id), ...this.calloutList.map(d => d.key)]);
    this.selected = new Set([...this.selected].filter(id => ids.has(id)));
    this.showSelection();
  }

  keepView() {
    this.onView?.(this.pan.x, this.pan.y, this.zoom);
  }

  // While the hand tool is on, the cards neither take a press nor show their own cursors
  showHand() {
    this.root.interactiveChildren = this.panning === null;
    // A press that has not moved yet may still be a click, so it keeps the plain cursor.
    // Not grab: that is the cursor of a card, which moves the card and not the view.
    const moving = this.panning !== null && this.panning.moved;
    this.app.stage.cursor = moving ? 'move' : 'default';
  }

  // The point of the card coordinates under a point of the screen, or null before PixiJS
  // has started
  toWorldX(clientX) {
    return this.world
      ? (clientX - this.app.canvas.getBoundingClientRect().left - this.world.x) / this.zoom
      : null;
  }

  toWorldY(clientY) {
    return this.world
      ? (clientY - this.app.canvas.getBoundingClientRect().top - this.world.y) / this.zoom
      : null;
  }

  // An unplaced card dragged over the canvas from the list beside it. With Ctrl, the card or
  // the area of a group under the pointer shows that the card would join its group, as for
  // cards dragged on the canvas.
  dragOverFromList(clientX, clientY, ctrl) {
    if (!this.world) {
      return;
    }
    this.listDrag ??= { origins: [], target: null, join: null };
    const e = this.listDragPointer(clientX, clientY, ctrl);
    this.showJoinGroup(this.listDrag, ctrl ? e : null);
    this.showGroupTarget(this.listDrag, this.groupTargetAt(this.listDrag, e));
  }

  // The unplaced card has left the canvas or been let go: nothing is shown as its group
  endDragFromList() {
    if (!this.listDrag) {
      return;
    }
    this.showJoinGroup(this.listDrag, null);
    this.showGroupTarget(this.listDrag, null);
    this.listDrag = null;
  }

  // The card whose group an unplaced card let go here with Ctrl joins: the card under the
  // pointer. Null without Ctrl.
  groupTargetFromList(clientX, clientY, ctrl) {
    if (!this.world || !ctrl) {
      return null;
    }
    const e = this.listDragPointer(clientX, clientY, ctrl);
    return this.groupTargetAt({ origins: [] }, e)?.cardData.id ?? null;
  }

  // The item an unplaced card let go here with Ctrl goes under: the one of the innermost group
  // whose area is under the pointer. Null without Ctrl, and on a card, whose group it joins.
  parentFromList(clientX, clientY, ctrl) {
    if (!this.world || !ctrl) {
      return null;
    }
    const drag = { origins: [] };
    const e = this.listDragPointer(clientX, clientY, ctrl);
    return this.groupTargetAt(drag, e) ? null : (this.joinAreaAt(drag, e)?.label ?? null);
  }

  // A point of the page as the pointer events of the canvas give it
  listDragPointer(clientX, clientY, ctrl) {
    const rect = this.app.canvas.getBoundingClientRect();
    return { global: new PIXI.Point(clientX - rect.left, clientY - rect.top), ctrlKey: ctrl };
  }

  // onGroup(cardIdsJson, targetId) puts the cards dropped with Ctrl on a card into its group
  setOnGroup(onGroup) {
    this.onGroup = onGroup;
  }

  // onShowText(cardId, clientX, clientY) shows the whole text of a card from its menu
  setOnShowText(onShowText) {
    this.onShowText = onShowText;
  }

  // onFollow(link, newTab, newWindow) follows a link in the text of a card. link is
  // { kind, url }: kind is Page, Web or File, as the core names the links of a page.
  setOnFollow(onFollow) {
    this.onFollow = onFollow;
  }

  // onMove(cardIdsJson, parentId) puts the cards let go with Ctrl in the area of another group
  // under the item of that group
  setOnMove(onMove) {
    this.onMove = onMove;
  }

  // onUngroup(cardIdsJson) takes the cards let go outside their group with Ctrl out of it
  setOnUngroup(onUngroup) {
    this.onUngroup = onUngroup;
  }

  // onScale(cardId, scale) scales a card
  setOnScale(onScale) {
    this.onScale = onScale;
  }

  // onGroupTitle(groupId, side, title) gives a group a title along a side of its outline
  setOnGroupTitle(onGroupTitle) {
    this.onGroupTitle = onGroupTitle;
  }

  // onAddCallout(x, y, side, text) adds a callout with its character centered at a point
  setOnAddCallout(onAddCallout) {
    this.onAddCallout = onAddCallout;
  }

  // onCallout(key, side, text) changes a callout; an empty text takes it away
  setOnCallout(onCallout) {
    this.onCallout = onCallout;
  }

  // onMoveCallout(key, x, y) moves a callout so the center of its character is at a point
  setOnMoveCallout(onMoveCallout) {
    this.onMoveCallout = onMoveCallout;
  }

  // onScaleCallout(key, scale) scales a callout around the center of its character
  setOnScaleCallout(onScaleCallout) {
    this.onScaleCallout = onScaleCallout;
  }

  // The callouts as JSON: [{ key, x, y, scale, side, text, character, drawSize, groups }], where
  // x and y are the center of the character, counted from the group the callout is right under,
  // side is the side of it the bubble is on, and character is an SVG of drawSize pixels square.
  // groups are the groups the callout is in, as those of a card.
  setCallouts(json) {
    this.calloutList = JSON.parse(json);
    if (this.started) {
      this.build();
    }
  }

  // The views of the callouts, each in the frame of its group
  buildCallouts() {
    for (const data of this.calloutList) {
      const scale = data.scale > 0 ? data.scale : 1;
      const view = callout(data, this.textResolution(scale), this.iconResolution() * Math.max(1, scale), this.fonts());
      view.extraId = data.key;
      view.extraLabel = view.calloutText;
      view.extraSelection = view.calloutSelection;
      view.saveMove = (x, y) => this.onMoveCallout?.(data.key, x, y);
      view.saveScale = scale => this.onScaleCallout?.(data.key, scale);
      if (this.editable) {
        this.makeDraggable(view, data);
      }
      view.frameNode = this.frameOf(groupsOf(data));
      view.frameNode.calloutLayer.addChild(view);
    }
  }

  // onAddLink(x, y, target) adds a link card with its center at a point; onSetLink(id, target)
  // links one to another page; onRemoveLink(id) takes one away, deleting its link. A link card
  // moves and scales as any card does. onLinkQuery(text) asks for the page names that match
  // what is typed in the dialog of a link, which come back through setLinkSuggestions; an
  // empty text clears them.
  setOnLink(onAddLink, onSetLink, onRemoveLink, onLinkQuery) {
    this.onAddLink = onAddLink;
    this.onSetLink = onSetLink;
    this.onRemoveLink = onRemoveLink;
    this.onLinkQuery = onLinkQuery;
  }

  // The page names for the open dialog of a link, as a JSON list of strings
  setLinkSuggestions(json) {
    this.showSuggestions?.(JSON.parse(json));
  }

  // The placed cards as JSON: [{ id, label, path, isDir, isLink, isPlain, tone, runs, x, y,
  // width, height, scale, group, groups, icon, blob, iconSize, iconDrawSize, masked }],
  // where x and y are the center of the card, counted from the group it is right under, or from
  // the center of the canvas in no group. groups are the groups a card is in, the outermost
  // first: [{ id, color, title, titleSide, path, icon, character, drawSize, x, y }], where x and
  // y are the place of the item of the group, counted from the group above it; group is the id
  // of the last of them. runs are the parts of the first line of a plain card: [{ text, link }],
  // where link is { kind, url } or null. A card with a null path opens nothing. A masked
  // card has its label hidden: its width is set, and only its blob shows, softened. The id of a link card is the id of its item, and its
  // path is the page it links to. width and height are its size as scaled, and a width of 0
  // leaves it to the label. icon and blob are SVGs or null, and iconDrawSize is the size of the
  // SVG, which is shown at iconSize. The core gives the color of each group, so it is the same
  // as on the tag icons.
  setCards(json) {
    this.data = JSON.parse(json);
    if (this.started) {
      this.build();
    }
  }

  // Everything on the canvas is made anew from the cards and the callouts: the frames of their
  // groups, nested as the groups are, and their views in those frames.
  build() {
    // The dragged card is destroyed with the others: a new list wins over the drag, of a
    // group too
    this.drag = null;
    if (this.groupDrag) {
      this.groupDrag = null;
      this.app.stage.cursor = 'default';
    }
    // The whole tree of frames goes, with the Graphics of each view; no two share a context.
    // The textures stay: the cards made next take the same ones.
    this.root.destroy({ children: true, context: true });
    this.root = this.world.addChild(frame(null));
    this.root.interactiveChildren = this.panning === null;
    this.frames = new Map();
    for (const data of this.data) {
      // The icons of a card scaled up are drawn in more pixels too
      const view = card(
        data,
        this.textResolution(data.scale),
        this.iconResolution() * Math.max(1, data.scale),
        this.fonts().label,
      );
      view.position.set(data.x, data.y);
      if (data.isLink) {
        view.linkData = data;
      }
      if (this.editable) {
        this.makeDraggable(view, data);
      } else {
        // The groups and the clicks find the data of a card here
        view.cardData = data;
      }
      view.frameNode = this.frameOf(groupsOf(data));
      view.frameNode.cardLayer.addChild(view);
    }
    this.buildCallouts();
    this.keepSelection();
    this.drawGroups();
  }

  // Each group is a smooth closed curve around all its cards and callouts, one Graphics for its
  // area and one for its frame, drawn in the frame of the group: its points are counted from the
  // place of the group, so the outline moves with the group and is not drawn again for it.
  // The frame is drawn in white and colored by its tint, so a change of its
  // color needs no new geometry. Given a drag, only the groups it changes are drawn again. With `ctrl`, the
  // outline of such a group stays as it was before the drag, with the dragged cards where
  // they were, so it shows whether they are let go in it. That shape does not change, so it
  // is drawn once while Ctrl is held; only a group dragged as a whole moves with its cards.
  // A group inside a group is drawn inside it: the outline of a group keeps a padding more
  // away from a card for each group between them, and lies behind the groups inside it.
  drawGroups(drag = null, ctrl = false) {
    const origins = new Map(drag ? [...drag.origins, ...(drag.extras ?? [])].map(o => [o.view, o]) : []);
    const dragged = new Set(origins.keys());
    const touched = drag ? this.groupsOfDrag(drag) : null;
    const views = [...this.cardViews(), ...this.extras()];
    const staying = new Set();
    for (const view of views) {
      if (!dragged.has(view)) {
        groupsOf(dataOf(view)).forEach(g => staying.add(g.id));
      }
    }
    const corners = new Map();
    const groups = new Map();
    for (const view of views) {
      const { width, height } = sizeOf(view);
      const chain = groupsOf(dataOf(view));
      chain.forEach((data, depth) => {
        const group = data.id;
        if (touched && !touched.has(group)) {
          return;
        }
        const kept = ctrl && staying.has(group);
        if (kept && drag.ctrlShape) {
          return;
        }
        // Where the view lies in the frame of this group: its place in its own frame, moved by
        // every frame between the two
        const from = kept && dragged.has(view) ? origins.get(view) : view;
        const { x, y } = placeIn(view.frameNode, this.frames.get(group), from.x, from.y);
        const padding = GROUP_PADDING * (chain.length - depth);
        const left = x - width / 2 - padding;
        const right = x + width / 2 + padding;
        const top = y - height / 2 - padding;
        const bottom = y + height / 2 + padding;
        if (!corners.has(group)) {
          corners.set(group, []);
          groups.set(group, data);
        }
        corners.get(group).push([left, top], [right, top], [right, bottom], [left, bottom]);
      });
    }
    if (!drag) {
      for (const node of this.frames.values()) {
        node.outline.removeChildren().forEach(c => c.destroy({ children: true, context: true }));
        node.area = null;
      }
    }
    for (const [group, points] of corners) {
      const node = this.frames.get(group);
      let area = node.area;
      if (!area) {
        area = new PIXI.Graphics({ label: group });
        // A press on the area drags the group. Not on its frame, which only shows the edge.
        area.eventMode = 'static';
        area.cursor = this.editable ? 'grab' : 'default';
        area.groupArea = true;
        area.frame = new PIXI.Graphics();
        // The frame shows only while a card of the group is dragged
        area.frame.visible = Boolean(drag && drag.moved);
        node.outline.addChild(area, area.frame);
        node.area = area;
      }
      const data = groups.get(group);
      const color = data.color;
      // One path for the area and its frame
      const curve = new PIXI.GraphicsPath();
      const hull = convexHull(points);
      smoothClosedCurve(curve, hull);
      area.clear().path(curve).fill({ color, alpha: 0.3 });
      area.frame.clear().path(curve).stroke({ color: 0xffffff, width: 2 });
      area.frame.tint = color;
      // Not `groupColor`: a Container uses that name for the tint it passes to its children,
      // and a color written there turns the area black while it is drawn again
      area.outlineColor = color;
      // A group dragged as a whole has nothing to leave: it moves with its cards
      area.leavable = ctrl && staying.has(group);
      const groupTitle = data.title;
      const groupTitleSide = data.titleSide;
      // A callout that cards are under stands along the outline in place of a title
      if (groupTitle && data.character) {
        area.calloutView ??= node.outline.addChild(this.groupCallout(data));
        area.calloutView.position.set(...besideOutline(hull, groupTitleSide));
        continue;
      }
      // One Text a character, made once; a drag only lays them again along the new outline
      if (groupTitle && !area.titleChars) {
        area.titleChars = new PIXI.Container();
        // The title of a page card opens its page, as the label of the card does
        area.titleChars.eventMode = data.path ? 'static' : 'none';
        area.titleChars.cursor = data.path ? 'pointer' : 'default';
        area.titleChars.titlePath = data.path ?? null;
        area.titleChars.alpha = TITLE_ALPHA;
        const style = new PIXI.TextStyle({
          fontFamily: this.fonts().title,
          fontSize: 20,
          fontWeight: 'bold',
          fill: 0x333333,
          // The same white edge as the labels of the cards
          stroke: { color: 0xffffff, width: TITLE_STROKE, join: 'round' },
          padding: 2,
        });
        // Where the outline crosses the characters in the box of a Text, as a share of its
        // height: TITLE_INSET of their height from the side toward the area. The canvas draws
        // them half the white edge down from the top of the box.
        const metrics = PIXI.CanvasTextMetrics.measureText(groupTitle, style);
        const { ascent, descent } = metrics.fontProperties;
        const line = share => (TITLE_STROKE / 2 + (ascent + descent) * share) / metrics.height;
        area.titleChars.standing = line(1 - TITLE_INSET);
        area.titleChars.hanging = line(TITLE_INSET);
        // How far each character moves the next one on, from the widths of the title up to it.
        // Not the width of the character alone: that holds the white edge at both its sides.
        const chars = Array.from(groupTitle);
        if (data.icon) {
          // The icon of the page stands before the title, as one more character
          const size = metrics.height;
          const icon = iconSprite(data.icon, size, size, this.iconResolution(), 0, 0, 1);
          icon.advance = size;
          area.titleChars.addChild(icon);
        }
        let before = 0;
        chars.forEach((char, i) => {
          const upTo = PIXI.CanvasTextMetrics.measureText(chars.slice(0, i + 1).join(''), style).width;
          const text = new PIXI.Text({ text: char, resolution: this.textResolution(), style });
          // A press on a character of a title that opens a page is a press on the title
          text.eventMode = data.path ? 'static' : 'none';
          text.advance = upTo - before;
          before = upTo;
          area.titleChars.addChild(text);
        });
        node.outline.addChild(area.titleChars);
      }
      if (area.titleChars) {
        layTitle(area.titleChars, closedCurvePoints(hull), groupTitleSide);
      }
    }
  }

  // The character of a callout that cards are under, with its text in a bubble. It is drawn as a
  // callout is, and only shows: it is moved and edited as its group.
  groupCallout(data) {
    const view = callout(
      {
        key: data.id,
        x: 0,
        y: 0,
        scale: 1,
        side: data.titleSide,
        text: data.title,
        character: data.character,
        drawSize: data.drawSize,
      },
      this.textResolution(),
      this.iconResolution(),
      this.fonts(),
    );
    view.eventMode = 'none';
    return view;
  }

  // The frame of a group shows while a card of it is dragged, so it is clear where the group
  // ends; at rest, and for the other groups, only the areas show. No drag hides every frame.
  showFrames(drag) {
    const dragged = drag ? this.groupsOfDrag(drag, true) : new Set();
    for (const area of this.areas()) {
      area.frame.visible = dragged.has(area.label);
    }
  }

  // While cards are dragged with Ctrl, the frame of a group they can leave takes the color of
  // a held group as long as the pointer is in its area, and its own color once they would
  // leave it. The area keeps its color. The groups they would leave are kept in the drag for
  // the drop. A pointer in the area of another group takes the cards out of their own, as
  // they join that group.
  showHeldGroups(drag, e) {
    drag.outside = new Set();
    for (const area of this.areas()) {
      if (!area.leavable) {
        continue;
      }
      // The drawn curve itself, not the hull it is made from: the curve swells out past the
      // edges of the hull
      const holding = !drag.join && area.containsPoint(area.toLocal(e.global));
      area.frame.tint = holding ? GROUP_HOLD_COLOR : area.outlineColor;
      if (!holding) {
        drag.outside.add(area.label);
      }
    }
  }

  // While cards are dragged with Ctrl, the area of another group under the pointer is the
  // group they would join: its frame shows, in the color of a held group. Without `e`, no
  // group is to be joined. The topmost area wins, as it is drawn over the others: of groups
  // one inside the other, the innermost.
  showJoinGroup(drag, e) {
    const join = e ? this.joinAreaAt(drag, e) : null;
    if (drag.join && drag.join !== join && !drag.join.destroyed) {
      drag.join.frame.visible = false;
      drag.join.frame.tint = drag.join.outlineColor;
    }
    if (join) {
      join.frame.visible = true;
      join.frame.tint = GROUP_HOLD_COLOR;
    }
    drag.join = join;
  }

  // The area of a group other than the dragged cards' own under the pointer, or null
  joinAreaAt(drag, e) {
    // Not a group the cards are in already, however deep
    const own = new Set(drag.origins.flatMap(o => groupsOf(o.view.cardData).map(g => g.id)));
    const areas = this.areas().filter(c => !own.has(c.label));
    for (let i = areas.length - 1; i >= 0; i--) {
      if (areas[i].containsPoint(areas[i].toLocal(e.global))) {
        return areas[i];
      }
    }
    return null;
  }

  // A drag moves the selected cards and callouts, each in the frame of its group, and places
  // them where they are let go. A click on the label of a card or a link card opens its page, in a new tab with Ctrl. A
  // click with Ctrl elsewhere adds it to the selection or takes it out. Only the cards take
  // part in the groups, the link cards too.
  makeDraggable(view, data) {
    // A callout has an id of its own; a card is selected by the id of its data
    const id = view.extraId ?? data.id;
    if (view.extraId === undefined) {
      view.cardData = data;
    }
    view.eventMode = 'static';
    view.cursor = 'grab';
    view.on('pointerdown', e => {
      // The right button is the hand tool; only the left button drags
      if (e.button !== 0) {
        return;
      }
      const ctrl = e.ctrlKey || e.metaKey;
      const wasSelected = this.selected.has(id);
      if (ctrl) {
        // Taken out only on release: a drag that starts here moves it with the others
        this.selected.add(id);
      } else if (!wasSelected) {
        this.selected = new Set([id]);
      }
      this.showSelection();
      const views = this.cardViews().filter(v => this.selected.has(v.cardData.id));
      const extras = this.extras().filter(v => this.selected.has(v.extraId));
      this.drag = {
        view,
        // Plain copies: `global` is reused for the next event
        start: { x: e.global.x, y: e.global.y },
        // The cards, which the groups follow, apart from the callouts
        origins: views.map(v => ({ view: v, x: v.x, y: v.y })),
        extras: extras.map(v => ({ view: v, x: v.x, y: v.y })),
        moved: false,
        ctrl,
        wasSelected,
        onLabel: Boolean(view.cardLabel) && e.target === view.cardLabel,
        runLink: runLinkOf(e.target),
      };
      // On top of the others of their frame while they move
      for (const v of [...views, ...extras]) {
        v.parent.addChild(v);
      }
    });
    view.on('globalpointermove', e => {
      const drag = this.drag;
      if (!drag || drag.view !== view) {
        return;
      }
      const dx = e.global.x - drag.start.x;
      const dy = e.global.y - drag.start.y;
      if (!drag.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) {
        return;
      }
      if (!drag.moved) {
        this.showFrames(drag);
      }
      drag.moved = true;
      view.cursor = 'grabbing';
      // The pointer moves in pixels of the screen, the cards in units of the scaled view. No
      // frame is scaled or turned, so a card moves as far in its frame as on the canvas.
      for (const o of [...drag.origins, ...drag.extras]) {
        o.view.position.set(o.x + dx / this.zoom, o.y + dy / this.zoom);
      }
      // Callouts alone touch no group
      const ctrl = (e.ctrlKey || e.metaKey) && drag.origins.length > 0;
      this.drawGroups(drag, ctrl);
      drag.ctrlShape = ctrl;
      if (ctrl) {
        this.showJoinGroup(drag, e);
        this.showHeldGroups(drag, e);
      } else {
        this.showJoinGroup(drag, null);
        drag.outside = null;
      }
      this.showGroupTarget(drag, ctrl ? this.groupTargetAt(drag, e) : null);
    });
    const end = e => {
      const drag = this.drag;
      if (!drag || drag.view !== view) {
        return;
      }
      this.drag = null;
      view.cursor = 'grab';
      if (drag.moved) {
        this.showFrames(null);
        const ctrl = (e.ctrlKey || e.metaKey) && drag.origins.length > 0;
        const target = ctrl ? this.groupTargetAt(drag, e) : null;
        this.showGroupTarget(drag, null);
        // The cards have moved with the pointer: they lie where they show. A position is the
        // center of a card, as the pivot is there, and the center of the character of a
        // callout. The core takes the place on the canvas, not the one in the frame: it counts
        // the place from the item the card is under, or will be. Read before placing any: placing
        // one gives a new list, which destroys these views.
        const at = view => onCanvas(view.frameNode, view.x, view.y);
        const places = drag.origins.map(o => [o.view.cardData.id, at(o.view).x, at(o.view).y]);
        const moves = drag.extras.map(o => [o.view.saveMove, at(o.view).x, at(o.view).y]);
        // A card let go on a card joins the group of that card; let go in the area of another
        // group, it goes under the item of that group
        const targetId = target ? target.cardData.id : null;
        const parentId = !targetId && ctrl && drag.join ? drag.join.label : null;
        // Joining a group takes the cards out of their own; only the cards let go outside their
        // group without joining another leave it
        const leavers =
          !targetId && !parentId && ctrl && drag.outside
            ? drag.origins.map(o => o.view.cardData).filter(d => drag.outside.has(d.group)).map(d => d.id)
            : [];
        for (const [id, x, y] of places) {
          this.onPlace(id, x, y);
        }
        for (const [save, x, y] of moves) {
          save(x, y);
        }
        if (targetId && this.onGroup) {
          this.onGroup(JSON.stringify(places.map(([id]) => id)), targetId);
        }
        if (parentId && this.onMove) {
          this.onMove(JSON.stringify(places.map(([id]) => id)), parentId);
        }
        if (leavers.length > 0 && this.onUngroup) {
          this.onUngroup(JSON.stringify(leavers));
        }
      } else if (drag.runLink && !data.masked) {
        // A link in the text of a card opens as a link of a page does
        this.onFollow?.(drag.runLink, drag.ctrl, e.shiftKey);
      } else if (drag.onLabel && opens(data)) {
        // A link, as it always was: Ctrl opens it in a new tab and leaves the selection
        // as it was before the press
        if (drag.ctrl && !drag.wasSelected) {
          this.selected.delete(id);
          this.showSelection();
        }
        this.onOpen(data.path, drag.ctrl, e.shiftKey);
      } else if (drag.ctrl && drag.wasSelected) {
        this.selected.delete(id);
        this.showSelection();
      } else if ((view.calloutData || view.linkData) && !drag.ctrl) {
        // A double click on a callout or a link card opens its dialog. The label of a link card
        // opens its page instead, as the label of a card does.
        const last = this.lastExtraClick;
        if (last && last.id === id && e.timeStamp - last.time < DOUBLE_CLICK_TIME) {
          this.lastExtraClick = null;
          if (view.linkData) {
            this.openLinkDialog(data);
          } else {
            this.openCalloutDialog(data);
          }
        } else {
          this.lastExtraClick = { id, time: e.timeStamp };
        }
      }
    };
    view.on('pointerup', end);
    view.on('pointerupoutside', end);
  }

  // The card under the pointer that the dragged cards would join, only while Ctrl is held.
  // The topmost one, as it is drawn over the others.
  groupTargetAt(drag, e) {
    if (!(e.ctrlKey || e.metaKey)) {
      return null;
    }
    const dragged = new Set(drag.origins.map(o => o.view));
    const cards = this.cardViews();
    for (let i = cards.length - 1; i >= 0; i--) {
      const v = cards[i];
      const p = v.toLocal(e.global);
      if (!dragged.has(v) && v.hitArea.contains(p.x, p.y)) {
        return v;
      }
    }
    return null;
  }

  showGroupTarget(drag, target) {
    if (drag.target === target) {
      return;
    }
    if (drag.target && !drag.target.destroyed) {
      drag.target.cardFrame.visible = false;
    }
    if (target) {
      target.cardFrame.visible = true;
    }
    drag.target = target;
  }

  // Whole pixels keep the lines sharp
  center() {
    this.world.scale.set(this.zoom);
    this.world.position.set(
      Math.round(this.app.screen.width / 2 + this.pan.x),
      Math.round(this.app.screen.height / 2 + this.pan.y),
    );
  }

  destroy() {
    this.destroyed = true;
    this.closeMenu();
    // The dialog listens on the window for its keys; a page left with it open would keep them
    this.closeDialog?.();
    clearTimeout(this.sharpenTimer);
    clearTimeout(this.scaleTimer);
    this.resizeObserver?.disconnect();
    if (this.root) {
      this.app.destroy({ removeView: true }, { children: true });
    }
  }
};

// The groups a card is in, the outermost first
function groupsOf(data) {
  return data?.groups ?? [];
}

// A frame: a Container that what lies in it is placed from. `outer` is the frame it lies in,
// null for the frame of the canvas. Its layers keep what is in it in order: the outline of its
// group lies behind the frames of the groups inside it, those behind its cards, and its
// callouts above them.
function frame(outer) {
  const node = new PIXI.Container();
  node.outer = outer;
  node.outline = new PIXI.Container();
  node.inner = new PIXI.Container();
  node.cardLayer = new PIXI.Container();
  node.calloutLayer = new PIXI.Container();
  node.addChild(node.outline, node.inner, node.cardLayer, node.calloutLayer);
  // The outline of its group, once it is drawn
  node.area = null;
  return node;
}

// A frame and every frame in it, itself first: the order their outlines are drawn in
function framesIn(node, out = []) {
  out.push(node);
  for (const inner of node.inner.children) {
    framesIn(inner, out);
  }
  return out;
}

// A point of the frame `from` as the frame `to` around it counts it: moved by the place of every
// frame from the one up to the other. No frame is scaled or turned, so the places only add up.
function placeIn(from, to, x, y) {
  for (let node = from; node && node !== to; node = node.outer) {
    x += node.x;
    y += node.y;
  }
  return { x, y };
}

// A point of a frame as the canvas counts it, from its center
function onCanvas(from, x, y) {
  return placeIn(from, null, x, y);
}

// The ids of the groups a frame is in, itself included; none for the frame of the canvas
function chainOf(node) {
  const ids = [];
  for (let n = node; n && n.outer; n = n.outer) {
    ids.push(n.groupId);
  }
  return ids;
}

// The data of the view of a card or of a callout
function dataOf(view) {
  return view.cardData ?? view.calloutData;
}

// How much room a view takes in the outline of its group: a card its size as scaled, a callout
// its character. The bubble of a callout may reach out of the outline.
function sizeOf(view) {
  if (view.cardData) {
    return { width: view.cardData.width, height: view.cardData.height };
  }
  const size = CALLOUT_SIZE * view.scale.x;
  return { width: size, height: size };
}

// The page a press on the title of a group opens, from a character of the title; none for a
// title that opens nothing
function titlePathOf(target) {
  for (let node = target; node; node = node.parent) {
    if (node.titlePath !== undefined) {
      return node.titlePath;
    }
  }
  return null;
}

// The link of the part of the text of a card that a press landed on; none off a link
function runLinkOf(target) {
  return target?.runLink ?? null;
}

// The card a press landed on, from the label or the frame inside it
function cardOf(target) {
  for (let node = target; node; node = node.parent) {
    if (node.cardData) {
      return node;
    }
  }
  return null;
}

// The callout a press landed on, from any part of it
function extraOf(target) {
  for (let node = target; node; node = node.parent) {
    if (node.extraId !== undefined) {
      return node;
    }
  }
  return null;
}

// A page name offered in the dialog of a link: a row the width of the input
function suggestionRow(title, width) {
  // Cut short, as a row does not wrap
  const text = Array.from(title).length > 44 ? `${Array.from(title).slice(0, 43).join('')}…` : title;
  return new PIXI.ui.FancyButton({
    defaultView: new PIXI.Graphics().roundRect(0, 0, width, LINK_ROW_HEIGHT - 2, 4).fill(0xffffff),
    hoverView: new PIXI.Graphics().roundRect(0, 0, width, LINK_ROW_HEIGHT - 2, 4).fill(0xe8f0fe),
    text: uiText(text),
    defaultTextAnchor: { x: 0, y: 0.5 },
    textOffset: { x: -width / 2 + 8, y: 0 },
  });
}

// Whether a display object lies inside a container
function within(target, container) {
  for (let node = target; node; node = node.parent) {
    if (node === container) {
      return true;
    }
  }
  return false;
}

function uiFont() {
  return getComputedStyle(document.body).fontFamily;
}

function uiText(text, fontSize = UI_FONT_SIZE) {
  return new PIXI.Text({ text, style: { fontFamily: uiFont(), fontSize, fill: 0x222222 } });
}

// A button of a dialog; the main one is filled with the color of the app
function dialogButton(text, main) {
  const view = (fill, line) =>
    new PIXI.Graphics()
      .roundRect(0, 0, 88, 32, 6)
      .fill(fill)
      .stroke({ color: line, width: 1 });
  return new PIXI.ui.FancyButton({
    defaultView: main ? view(0x1a73e8, 0x1a73e8) : view(0xffffff, 0xc0c4cc),
    hoverView: main ? view(0x1765cc, 0x1765cc) : view(0xf0f0f0, 0xc0c4cc),
    pressedView: main ? view(0x1558b0, 0x1558b0) : view(0xe4e4e4, 0xc0c4cc),
    text: new PIXI.Text({
      text,
      style: { fontFamily: uiFont(), fontSize: UI_FONT_SIZE, fill: main ? 0xffffff : 0x222222 },
    }),
  });
}

// The colors of the labels: a link out of the folder in blue, as it always was, and a link to
// a page that is not there in gray
const LABEL_COLORS = { link: LINK_LABEL_COLOR, missing: 0x888888 };

// The label of a card: its text in one Text, wrapped to the width of the card. The text of a
// plain card with links in it is a row of its parts instead, each link a part of its own that a
// click follows. That row does not wrap: a long first line is for the user to shorten.
function cardText(data, resolution, fontFamily, setWidth) {
  const style = fill => ({
    fontFamily,
    fontSize: 13,
    fill,
    // A white edge keeps the label readable over the icon and the groups behind it. The
    // padding keeps the edge from being cut off at the sides of the text.
    stroke: { color: 0xffffff, width: 3, join: 'round' },
    padding: 2,
  });
  const color = LABEL_COLORS[data.tone] ?? 0x222222;
  const runs = data.runs ?? [];
  if (!runs.some(run => run.link)) {
    return new PIXI.Text({
      text: data.label,
      resolution,
      style: {
        ...style(color),
        wordWrap: true,
        wordWrapWidth: Math.max(1, (setWidth ?? CARD_MAX_WIDTH) - 2 * CARD_PADDING),
        breakWords: true,
        align: 'center',
      },
    });
  }
  const row = new PIXI.Container();
  let x = 0;
  for (const run of runs) {
    const part = new PIXI.Text({ text: run.text, resolution, style: style(run.link ? LINK_LABEL_COLOR : color) });
    part.position.set(x, 0);
    if (run.link) {
      part.eventMode = 'static';
      part.cursor = 'pointer';
      part.runLink = run.link;
    }
    x += part.width;
    row.addChild(part);
  }
  return row;
}

// A group title stands on the outline, one character after another, each turned with the
// curve where it stands: the line crosses its characters a third of the way up, so they reach
// a little into the area. It is centered on the middle of the side it names. It runs clockwise on the screen, so it reads left to right along the
// top, downward on the right and upward on the left, its foot toward the outline. Along the
// bottom it runs the other way, so it is not upside down; there it hangs from the line by its
// head, as standing on it would put it inside the group.
function layTitle(title, outline, side) {
  const chars = title.children;
  if (outline.length < 2) {
    return;
  }
  // Clockwise on the screen, where y grows downward, has a positive signed area
  let area = 0;
  outline.forEach(([x, y], i) => {
    const [nx, ny] = outline[(i + 1) % outline.length];
    area += x * ny - nx * y;
  });
  const bottom = side === 'bottom';
  const path = (area > 0) !== bottom ? outline : [...outline].reverse();
  const lengths = [0];
  for (let i = 1; i <= path.length; i++) {
    const [ax, ay] = path[i - 1];
    const [bx, by] = path[i % path.length];
    lengths.push(lengths[i - 1] + Math.hypot(bx - ax, by - ay));
  }
  const total = lengths[path.length];
  // The point of the outline at a distance along it, from its first point
  const at = distance => {
    const d = ((distance % total) + total) % total;
    let i = 1;
    while (lengths[i] < d) {
      i++;
    }
    const t = (d - lengths[i - 1]) / (lengths[i] - lengths[i - 1] || 1);
    const [ax, ay] = path[i - 1];
    const [bx, by] = path[i % path.length];
    return [ax + (bx - ax) * t, ay + (by - ay) * t];
  };
  // The middle of the side: where a line from the middle of the outline toward that side
  // crosses it. Not the farthest point, which on a flat side may lie at one of its ends.
  const xs = path.map(([x]) => x);
  const ys = path.map(([, y]) => y);
  const midX = (Math.min(...xs) + Math.max(...xs)) / 2;
  const midY = (Math.min(...ys) + Math.max(...ys)) / 2;
  const across = side === 'left' || side === 'right' ? ([, y]) => Math.abs(y - midY) : ([x]) => Math.abs(x - midX);
  const outward = {
    top: ([, y]) => y < midY,
    bottom: ([, y]) => y > midY,
    left: ([x]) => x < midX,
    right: ([x]) => x > midX,
  }[side] ?? (([, y]) => y < midY);
  let middle = -1;
  path.forEach((p, i) => {
    if (outward(p) && (middle < 0 || across(p) < across(path[middle]))) {
      middle = i;
    }
  });
  if (middle < 0) {
    middle = 0;
  }
  const width = chars.reduce((sum, c) => sum + c.advance, 0);
  let distance = lengths[middle] - width / 2;
  for (const char of chars) {
    const step = char.advance;
    // The direction over the width of the character, so it does not follow every small bend
    const [ax, ay] = at(distance);
    const [bx, by] = at(distance + step);
    const [cx, cy] = at(distance + step / 2);
    char.anchor.set(0.5, bottom ? title.hanging : title.standing);
    char.rotation = Math.atan2(by - ay, bx - ax);
    char.position.set(cx, cy);
    distance += step;
  }
}

// Where the character of a callout stands beside an outline: just outside the middle of the
// side its bubble is on, so the bubble faces away from the group
function besideOutline(hull, side) {
  const xs = hull.map(([x]) => x);
  const ys = hull.map(([, y]) => y);
  const [left, right, top, bottom] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const midX = (left + right) / 2;
  const midY = (top + bottom) / 2;
  const away = CALLOUT_SIZE / 2;
  return (
    {
      bottom: [midX, bottom + away],
      left: [left - away, midY],
      right: [right + away, midY],
    }[side] ?? [midX, top - away]
  );
}

// Points along the closed curve of smoothClosedCurve, close enough to stand for it
function closedCurvePoints(points) {
  const out = [];
  for (const [p1, c1, c2, p2] of curveSegments(points)) {
    for (let k = 0; k < CURVE_SAMPLES; k++) {
      const t = k / CURVE_SAMPLES;
      const u = 1 - t;
      const a = u * u * u;
      const b = 3 * u * u * t;
      const c = 3 * u * t * t;
      const d = t * t * t;
      out.push([
        a * p1[0] + b * c1[0] + c * c2[0] + d * p2[0],
        a * p1[1] + b * c1[1] + c * c2[1] + d * p2[1],
      ]);
    }
  }
  return out;
}

// The smallest convex polygon around the points, in order around it (Andrew's monotone chain)
function convexHull(points) {
  const sorted = [...points].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const turn = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const half = list => {
    const out = [];
    for (const p of list) {
      while (out.length >= 2 && turn(out[out.length - 2], out[out.length - 1], p) <= 0) {
        out.pop();
      }
      out.push(p);
    }
    out.pop();
    return out;
  };
  return [...half(sorted), ...half([...sorted].reverse())];
}

// A closed curve through every point, drawn on a Graphics or a GraphicsPath, as Bézier curves of a Catmull-Rom spline. It passes
// through the corners of the hull, so it never cuts into a card as a curve between them could.
function smoothClosedCurve(g, points) {
  g.moveTo(points[0][0], points[0][1]);
  for (const [, c1, c2, p2] of curveSegments(points)) {
    g.bezierCurveTo(c1[0], c1[1], c2[0], c2[1], p2[0], p2[1]);
  }
  g.closePath();
}

// The Bézier curves of the closed Catmull-Rom spline through the points: [start, control,
// control, end] each
function curveSegments(points) {
  const n = points.length;
  const at = i => points[(i + n) % n];
  const segments = [];
  for (let i = 0; i < n; i++) {
    const [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
    segments.push([
      p1,
      [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6],
      [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6],
      p2,
    ]);
  }
  return segments;
}

// The icon of a card, from its SVG. The browser draws the SVG, as its hatching is a <pattern>,
// which a Graphics cannot read. The texture comes later; the card shows without it until then.
function iconSprite(svg, width, height, resolution, x, y, alpha) {
  const sprite = new PIXI.Sprite();
  sprite.anchor.set(0.5);
  sprite.position.set(x, y);
  sprite.alpha = alpha;
  // The card handles the press
  sprite.eventMode = 'none';
  PIXI.Assets.load({
    src: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`,
    parser: 'svg',
    data: { resolution },
  })
    .then(texture => {
      if (!sprite.destroyed) {
        sprite.texture = texture;
        sprite.setSize(width, height);
      }
    })
    .catch(e => console.warn('atlas: cannot draw a tag icon', e));
  return sprite;
}

// A callout: its character centered at its point, and a speech bubble on one side of it with
// the tail toward the character. The character faces the other way when its bubble is on the
// left or below it.
// Whether a click on the label of a card opens a page
function opens(data) {
  return Boolean(data?.path) && !data.masked;
}

function callout(data, resolution, iconResolution, fonts) {
  const { x, y, side, text, character, drawSize } = data;
  const view = new PIXI.Container();
  view.position.set(x, y);
  view.calloutData = data;
  view.scale.set(data.scale > 0 ? data.scale : 1);
  // A holder is flipped, not the sprite: the sprite sets its own scale once its texture comes
  const holder = new PIXI.Container();
  holder.scale.x = side === 'left' || side === 'bottom' ? -1 : 1;
  holder.addChild(
    iconSprite(character, CALLOUT_SIZE, CALLOUT_SIZE, (iconResolution * CALLOUT_SIZE) / drawSize, 0, 0, 1),
  );
  // The face takes the press, not the margin of its SVG. On the holder, not the callout: a
  // hit area on the callout would keep the bubble from taking a press.
  holder.eventMode = 'static';
  holder.hitArea = new PIXI.Circle(0, 0, CALLOUT_REACH);
  const label = new PIXI.Text({
    text,
    resolution,
    style: {
      fontFamily: fonts.title,
      fontSize: CALLOUT_FONT_SIZE,
      fill: 0x333333,
      wordWrap: true,
      wordWrapWidth: BUBBLE_TEXT_WIDTH,
      breakWords: true,
    },
  });
  label.anchor.set(0.5);
  const w = label.width + 2 * BUBBLE_PADDING;
  const h = label.height + 2 * BUBBLE_PADDING;
  // The center of the bubble, from the center of the character
  const away = CALLOUT_REACH + BUBBLE_GAP + BUBBLE_TAIL;
  const [bx, by] = {
    top: [0, -(away + h / 2)],
    bottom: [0, away + h / 2],
    left: [-(away + w / 2), 0],
    right: [away + w / 2, 0],
  }[side] ?? [0, -(away + h / 2)];
  const bubble = new PIXI.Graphics();
  sharpCornerCurve(bubble, wobble(bubbleOutline(w, h, side), hashText(data.key)));
  bubble.fill(0xffffff).stroke({ color: 0x333333, width: 1.5 });
  bubble.position.set(bx, by);
  label.position.set(bx, by);
  // The bubble takes the press too
  bubble.eventMode = 'static';
  // A ring around the face and the bubble, as around a selected card
  const left = Math.min(-CALLOUT_SIZE / 2, bx - w / 2) - 4;
  const top = Math.min(-CALLOUT_SIZE / 2, by - h / 2) - 4;
  const right = Math.max(CALLOUT_SIZE / 2, bx + w / 2) + 4;
  const bottom = Math.max(CALLOUT_SIZE / 2, by + h / 2) + 4;
  view.calloutSelection = new PIXI.Graphics()
    .roundRect(left, top, right - left, bottom - top, 10)
    .stroke({ color: 0x3b82f6, width: 2 });
  view.calloutSelection.visible = false;
  view.calloutText = label;
  view.addChild(view.calloutSelection, holder, bubble, label);
  return view;
}

// The outline of a bubble of the size, centered at the origin, as points running clockwise: a
// rounded box with a tail out of the side that faces the character. That is the bottom of a
// bubble above the character, the right of one on its left, and so on. One closed line, so it
// runs around the tail and not across it. The tip of the tail is marked, so it stays sharp.
function bubbleOutline(w, h, side) {
  const r = Math.min(8, w / 2, h / 2);
  const hw = w / 2;
  const hh = h / 2;
  const base = Math.min(BUBBLE_TAIL_BASE, w - 2 * r, h - 2 * r) / 2;
  const t = BUBBLE_TAIL;
  // The edge the tail leaves from, as the line runs clockwise
  const edge = { bottom: 'top', left: 'right', top: 'bottom', right: 'left' }[side] ?? 'bottom';
  const tails = {
    top: [[-base, -hh], [0, -hh - t, true], [base, -hh]],
    right: [[hw, -base], [hw + t, 0, true], [hw, base]],
    bottom: [[base, hh], [0, hh + t, true], [-base, hh]],
    left: [[-hw, base], [-hw - t, 0, true], [-hw, -base]],
  };
  // A quarter circle from the angle, in degrees; y grows downward, so it turns clockwise
  const corner = (cx, cy, from) =>
    [0, 30, 60, 90].map(d => {
      const a = ((from + d) * Math.PI) / 180;
      return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
    });
  const vertices = [
    ...(edge === 'top' ? tails.top : []),
    ...corner(hw - r, -hh + r, -90),
    ...(edge === 'right' ? tails.right : []),
    ...corner(hw - r, hh - r, 0),
    ...(edge === 'bottom' ? tails.bottom : []),
    ...corner(-hw + r, hh - r, 90),
    ...(edge === 'left' ? tails.left : []),
    ...corner(-hw + r, -hh + r, 180),
  ];
  // Points every BUBBLE_STEP or so along the straight runs, so the line has room to wander
  const points = [];
  vertices.forEach((v, i) => {
    const [nx, ny] = vertices[(i + 1) % vertices.length];
    const n = Math.max(1, Math.round(Math.hypot(nx - v[0], ny - v[1]) / BUBBLE_STEP));
    points.push(v);
    for (let k = 1; k < n; k++) {
      points.push([v[0] + ((nx - v[0]) * k) / n, v[1] + ((ny - v[1]) * k) / n]);
    }
  });
  return points;
}

// The points of a clockwise outline, each moved out or in along the outline's normal by a slow
// wave, so the line looks drawn by hand. The wave is random, from the seed: a callout keeps its
// shape each time it is drawn. It goes round a whole number of times, so it meets itself where
// the outline closes. A marked point stays where it is, and stays marked.
function wobble(points, seed) {
  const random = seededRandom(seed);
  const lengths = [0];
  for (let i = 1; i < points.length; i++) {
    const [ax, ay] = points[i - 1];
    const [bx, by] = points[i];
    lengths.push(lengths[i - 1] + Math.hypot(bx - ax, by - ay));
  }
  const [lx, ly] = points[points.length - 1];
  const total = lengths[points.length - 1] + Math.hypot(points[0][0] - lx, points[0][1] - ly);
  const waves = [
    [3 + Math.floor(random() * 3), random() * 2 * Math.PI, 0.6],
    [8 + Math.floor(random() * 5), random() * 2 * Math.PI, 0.4],
  ];
  return points.map((p, i) => {
    if (p[2]) {
      return p;
    }
    const [px, py] = points[(i - 1 + points.length) % points.length];
    const [nx, ny] = points[(i + 1) % points.length];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const along = (lengths[i] / total) * 2 * Math.PI;
    const offset = BUBBLE_WOBBLE * waves.reduce((sum, [k, phase, share]) => sum + share * Math.sin(k * along + phase), 0);
    // Outward from a clockwise line on the screen is to its left
    return [p[0] + ((ny - py) / length) * offset, p[1] - ((nx - px) / length) * offset];
  });
}

// A closed curve through every point, as smoothClosedCurve draws it, but with a corner at each
// marked point: the curves on both sides of it leave it straight toward their neighbors.
function sharpCornerCurve(g, points) {
  const n = points.length;
  const at = i => points[(i + n) % n];
  g.moveTo(points[0][0], points[0][1]);
  for (let i = 0; i < n; i++) {
    const [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
    const c1 = p1[2] ? p1 : [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = p2[2] ? p2 : [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    g.bezierCurveTo(c1[0], c1[1], c2[0], c2[1], p2[0], p2[1]);
  }
  g.closePath();
}

// A hash of a text, as a seed
function hashText(text) {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash = Math.imul(hash ^ text.charCodeAt(i), 0x01000193);
  }
  return hash >>> 0;
}

// Numbers from 0 to 1 that are the same for the same seed (mulberry32)
function seededRandom(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// The SVG with its own size set to the given one; its view box is stretched to fit
function sizedSvg(svg, width, height) {
  return svg.replace(/width="[^"]*" height="[^"]*"/, `width="${width}" height="${height}"`);
}

function cross() {
  return new PIXI.Graphics()
    .moveTo(-CROSS_SIZE, -CROSS_SIZE)
    .lineTo(CROSS_SIZE, CROSS_SIZE)
    .moveTo(CROSS_SIZE, -CROSS_SIZE)
    .lineTo(-CROSS_SIZE, CROSS_SIZE)
    .stroke({ color: 0x999999, width: 1.5 });
}

// A card with a stored width of 0 is as wide as its label, between the width of its icon and
// CARD_MAX_WIDTH; a longer name wraps. Any other stored width is the width of the card, and the
// name wraps to fit it. The card is drawn at its unscaled size and then scaled as a whole. The
// width it takes on the canvas is written back into the data, which the outlines of the
// groups read.
function card(data, resolution, iconResolution, fontFamily) {
  const { icon, blob, iconSize, iconDrawSize, masked } = data;
  const scale = data.scale > 0 ? data.scale : 1;
  const height = data.height / scale;
  const setWidth = data.width > 0 ? data.width / scale : null;
  const text = cardText(data, resolution, fontFamily, setWidth);
  // A canvas Text knows its size as soon as it is made
  const width =
    setWidth ??
    Math.ceil(Math.max(icon ? iconSize : 0, text.width + 2 * CARD_PADDING, CARD_MIN_WIDTH));
  data.width = width * scale;
  const c = new PIXI.Container();
  c.scale.set(scale);
  // The unscaled size, which a new scale multiplies
  c.cardWidth = width;
  c.cardHeight = height;
  // The position of a card is its center, as stored in atlas_bounds
  c.pivot.set(width / 2, height / 2);
  // A ring outside the frame, so it does not cover the frame's own color
  c.cardSelection = new PIXI.Graphics()
    .roundRect(-3, -3, width + 6, height + 6, 10)
    .stroke({ color: 0x3b82f6, width: 2 });
  c.cardSelection.visible = false;
  c.addChild(c.cardSelection);
  // A card has no line and no background; only the target of a Ctrl drop shows one. The whole
  // card still takes a press, though it draws nothing between its blob and its edges.
  c.hitArea = new PIXI.Rectangle(0, 0, width, height);
  c.cardFrame = new PIXI.Graphics()
    .roundRect(0, 0, width, height, 8)
    .fill({ color: GROUP_TARGET_TINT, alpha: CARD_ALPHA });
  c.cardFrame.visible = false;
  c.addChild(c.cardFrame);
  // The icon and the label share the inside of the card, which neither may spill out of: an
  // icon may be taller than the card, and a wrapped name taller too
  const inside = new PIXI.Container();
  inside.mask = new PIXI.Graphics().roundRect(0, 0, width, height, 8).fill(0xffffff);
  c.addChild(inside.mask, inside);
  if (blob) {
    // Over the whole card, behind the icon. The SVG is drawn at the size of the card rather than
    // drawn square and stretched: a stretched line around the blob would be thicker on two sides.
    const alpha = masked ? MASKED_ALPHA : 1;
    inside.addChild(iconSprite(sizedSvg(blob, width, height), width, height, iconResolution, width / 2, height / 2, alpha));
  }
  if (icon) {
    // Rasterized in as many pixels as the icon covers at its largest, not at the size of the SVG
    const rasterResolution = (iconResolution * iconSize) / iconDrawSize;
    // Behind the label, in the middle of the card. Faint, so it stays behind the label.
    inside.addChild(
      iconSprite(icon, iconSize, iconSize, rasterResolution, width / 2, height / 2, ICON_ALPHA),
    );
  }
  if (text.anchor) {
    text.anchor.set(0.5);
  } else {
    text.pivot.set(text.width / 2, text.height / 2);
  }
  text.position.set(width / 2, height / 2);
  // The card handles the press; the label only shows that it opens the page
  text.eventMode = 'static';
  text.cursor = opens(data) ? 'pointer' : 'default';
  c.cardLabel = text;
  inside.addChild(text);
  return c;
}
