// The canvas of an atlas page: the cards of the pages and folders of its directory, drawn
// with PixiJS. PixiJS is the global `PIXI` of vendor/pixi.min.js. The origin of the card
// coordinates is the center of the canvas, marked by a gray cross. It stays in the center
// when the host is resized, so the cards keep their places around it.
//
// The hand tool moves the view: a drag with the right button, or with the left one
// between the cards. It moves the container of the cards and the cross, not the cards.
// The wheel scales that container around the point under the pointer.
//
// A plain script that sets the global `AtlasCanvas`, loaded by index.html. Not a module
// of wasm-bindgen: Trunk puts a hash of the content in the name of a script it loads, so a
// browser never keeps an old copy. A module keeps the same name when it changes.

const CROSS_SIZE = 8;
// A press that moves less than this is a click, not a drag
const DRAG_THRESHOLD = 3;
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
// The color a card takes while cards held with Ctrl are over it, to join its group
const GROUP_TARGET_TINT = 0xbfdbfe;
// How far the outline of a group keeps away from the cards in it
const GROUP_PADDING = 14;
// The colors of the groups, picked by the key of each group
const GROUP_COLORS = [0x60a5fa, 0x34d399, 0xf59e0b, 0xf472b6, 0xa78bfa, 0x2dd4bf, 0xfb7185, 0xa3e635];

window.AtlasCanvas = class AtlasCanvas {
  // onOpen(path, newTab, newWindow) opens the page of a card whose label is clicked.
  // onPlace(pageId, x, y) places the card of a page with its center at a point.
  // onMenu(pageId, clientX, clientY) opens the menu of a card that is right-clicked.
  // onView(panX, panY, zoom) keeps the view once the hand tool or the wheel has stopped.
  // panX, panY and zoom are the view to start with, as onView last gave them.
  constructor(host, onOpen, onPlace, onMenu, onView, panX, panY, zoom) {
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
    this.destroyed = false;
    this.app = new PIXI.Application();
    this.ready = this.app
      .init({
        resizeTo: host,
        background: '#f5f6f8',
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
        this.cards = new PIXI.Container();
        // The outlines of the groups lie behind the cards
        this.groups = new PIXI.Graphics();
        this.world.addChild(cross(), this.groups, this.cards);
        this.app.stage.addChild(this.world);
        this.app.renderer.on('resize', () => this.center());
        // resizeTo follows the window only. The host also changes size when the list of
        // unplaced cards beside it fills or empties, which would leave the center off.
        this.resizeObserver = new ResizeObserver(() => this.app.resize());
        this.resizeObserver.observe(host);
        this.listenForHand();
        this.center();
        this.build();
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
  }

  // The point under the pointer stays where it is while the view is scaled
  zoomAt(e) {
    e.preventDefault();
    const rect = this.app.canvas.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    const delta = e.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? e.deltaY : e.deltaY * LINE_HEIGHT;
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, this.zoom * Math.exp(-delta * ZOOM_PER_PIXEL)));
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
    // Drawing every label again on each step of the wheel would make the zoom stutter.
    // The view is kept then too, not on each step.
    clearTimeout(this.sharpenTimer);
    this.sharpenTimer = setTimeout(() => {
      this.sharpenText();
      this.keepView();
    }, SHARPEN_DELAY);
  }

  // The labels are drawn in as many pixels as they cover on the screen, so they stay sharp
  // when the view is scaled up. Not ahead of time for the largest zoom: every label would
  // then hold a texture many times its size.
  textResolution() {
    const zoom = Math.min(MAX_TEXT_ZOOM, Math.max(1, this.zoom));
    return (window.devicePixelRatio || 1) * zoom;
  }

  sharpenText() {
    const resolution = this.textResolution();
    for (const view of this.cards.children) {
      if (view.cardLabel.resolution !== resolution) {
        view.cardLabel.resolution = resolution;
      }
    }
  }

  startPan(e) {
    // The left button on a card drags the card; between the cards it moves the view
    const card = cardOf(e.target);
    const hand = e.button === 2 || (e.button === 0 && !card);
    if (!hand || this.panning) {
      return;
    }
    this.panning = {
      button: e.button,
      start: { x: e.global.x, y: e.global.y },
      origin: { x: this.pan.x, y: this.pan.y },
      moved: false,
      // A right click that does not move opens the menu of this card
      card,
    };
    this.showHand();
  }

  movePan(e) {
    const panning = this.panning;
    if (!panning) {
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
    const panning = this.panning;
    if (!panning || e.button !== panning.button) {
      return;
    }
    this.panning = null;
    this.showHand();
    if (panning.moved) {
      this.keepView();
    } else if (panning.button === 2 && panning.card) {
      this.onMenu(panning.card.cardData.id, e.clientX, e.clientY);
    } else if (panning.button === 0 && !panning.card) {
      // A click between the cards lets go of the selection
      this.select([]);
    }
  }

  select(ids) {
    this.selected = new Set(ids);
    this.showSelection();
  }

  showSelection() {
    for (const view of this.cards.children) {
      view.cardSelection.visible = this.selected.has(view.cardData.id);
    }
  }

  keepView() {
    this.onView(this.pan.x, this.pan.y, this.zoom);
  }

  // While the hand tool is on, the cards neither take a press nor show their own cursors
  showHand() {
    this.cards.interactiveChildren = this.panning === null;
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

  // onGroup(pageIdsJson, targetId) puts the cards dropped with Ctrl on a card into its group
  setOnGroup(onGroup) {
    this.onGroup = onGroup;
  }

  // The placed cards as JSON: [{ id, label, path, isDir, x, y, width, height }], where x
  // and y are the center of the card
  setCards(json) {
    this.data = JSON.parse(json);
    if (this.cards) {
      this.build();
    }
  }

  build() {
    // The dragged card is destroyed with the others: a new list wins over the drag
    this.drag = null;
    this.cards.removeChildren().forEach(c => c.destroy({ children: true }));
    for (const data of this.data) {
      const view = card(data, this.textResolution());
      view.position.set(data.x, data.y);
      this.makeDraggable(view, data);
      this.cards.addChild(view);
    }
    // A card taken off the canvas is no longer selected
    this.selected = new Set(this.data.map(d => d.id).filter(id => this.selected.has(id)));
    this.showSelection();
    this.drawGroups();
  }

  // Each group is a smooth closed curve around all its cards, drawn again as they move
  drawGroups() {
    this.groups.clear();
    const corners = new Map();
    for (const view of this.cards.children) {
      const { group, width, height } = view.cardData;
      if (!group) {
        continue;
      }
      const left = view.x - width / 2 - GROUP_PADDING;
      const right = view.x + width / 2 + GROUP_PADDING;
      const top = view.y - height / 2 - GROUP_PADDING;
      const bottom = view.y + height / 2 + GROUP_PADDING;
      if (!corners.has(group)) {
        corners.set(group, []);
      }
      corners.get(group).push([left, top], [right, top], [right, bottom], [left, bottom]);
    }
    for (const [group, points] of corners) {
      const color = GROUP_COLORS[hashOf(group) % GROUP_COLORS.length];
      smoothClosedCurve(this.groups, convexHull(points));
      this.groups.fill({ color, alpha: 0.12 }).stroke({ color, alpha: 0.7, width: 2 });
    }
  }

  // A drag moves the selected cards and places them where they are let go. A click on the
  // label opens the page, in a new tab with Ctrl. A click with Ctrl elsewhere on the card
  // adds the card to the selection or takes it out.
  makeDraggable(view, data) {
    view.cardData = data;
    view.eventMode = 'static';
    view.cursor = 'grab';
    view.on('pointerdown', e => {
      // The right button is the hand tool; only the left button drags a card
      if (e.button !== 0) {
        return;
      }
      const ctrl = e.ctrlKey || e.metaKey;
      const wasSelected = this.selected.has(data.id);
      if (ctrl) {
        // Taken out only on release: a drag that starts here moves it with the others
        this.selected.add(data.id);
      } else if (!wasSelected) {
        this.selected = new Set([data.id]);
      }
      this.showSelection();
      const views = this.cards.children.filter(v => this.selected.has(v.cardData.id));
      this.drag = {
        view,
        // Plain copies: `global` is reused for the next event
        start: { x: e.global.x, y: e.global.y },
        origins: views.map(v => ({ view: v, x: v.x, y: v.y })),
        moved: false,
        ctrl,
        wasSelected,
        onLabel: e.target === view.cardLabel,
      };
      // On top of the others while they move
      for (const v of views) {
        this.cards.addChild(v);
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
      drag.moved = true;
      view.cursor = 'grabbing';
      // The pointer moves in pixels of the screen, the cards in units of the scaled view
      for (const o of drag.origins) {
        o.view.position.set(o.x + dx / this.zoom, o.y + dy / this.zoom);
      }
      this.drawGroups();
      this.showGroupTarget(drag, this.groupTargetAt(drag, e));
    });
    const end = e => {
      const drag = this.drag;
      if (!drag || drag.view !== view) {
        return;
      }
      this.drag = null;
      view.cursor = 'grab';
      if (drag.moved) {
        const target = this.groupTargetAt(drag, e);
        this.showGroupTarget(drag, null);
        // The cards have moved with the pointer: they lie where they show. A position is the
        // center of a card, as the pivot is there. Read before placing any: placing one
        // gives a new list of cards, which destroys these views.
        const places = drag.origins.map(o => [o.view.cardData.id, o.view.x, o.view.y]);
        const targetId = target && target.cardData.id;
        for (const [id, x, y] of places) {
          this.onPlace(id, x, y);
        }
        if (targetId && this.onGroup) {
          this.onGroup(JSON.stringify(places.map(([id]) => id)), targetId);
        }
      } else if (drag.onLabel) {
        // A link, as it always was: Ctrl opens it in a new tab and leaves the selection
        // as it was before the press
        if (drag.ctrl && !drag.wasSelected) {
          this.selected.delete(data.id);
          this.showSelection();
        }
        this.onOpen(data.path, drag.ctrl, e.shiftKey);
      } else if (drag.ctrl && drag.wasSelected) {
        this.selected.delete(data.id);
        this.showSelection();
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
    const cards = this.cards.children;
    for (let i = cards.length - 1; i >= 0; i--) {
      const v = cards[i];
      if (!dragged.has(v) && v.cardFrame.getBounds().containsPoint(e.global.x, e.global.y)) {
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
      drag.target.cardFrame.tint = 0xffffff;
    }
    if (target) {
      target.cardFrame.tint = GROUP_TARGET_TINT;
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
    clearTimeout(this.sharpenTimer);
    this.resizeObserver?.disconnect();
    if (this.cards) {
      this.app.destroy({ removeView: true }, { children: true });
    }
  }
};

// The card a press landed on, from the label or the frame inside it
function cardOf(target) {
  for (let node = target; node; node = node.parent) {
    if (node.cardData) {
      return node;
    }
  }
  return null;
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

// A closed curve through every point, as Bézier curves of a Catmull-Rom spline. It passes
// through the corners of the hull, so it never cuts into a card as a curve between them could.
function smoothClosedCurve(g, points) {
  const n = points.length;
  const at = i => points[(i + n) % n];
  g.moveTo(points[0][0], points[0][1]);
  for (let i = 0; i < n; i++) {
    const [p0, p1, p2, p3] = [at(i - 1), at(i), at(i + 1), at(i + 2)];
    g.bezierCurveTo(
      p1[0] + (p2[0] - p0[0]) / 6,
      p1[1] + (p2[1] - p0[1]) / 6,
      p2[0] - (p3[0] - p1[0]) / 6,
      p2[1] - (p3[1] - p1[1]) / 6,
      p2[0],
      p2[1],
    );
  }
  g.closePath();
}

// A stable small number from the key of a group, to pick its color
function hashOf(text) {
  let hash = 0;
  for (const ch of text) {
    hash = (hash * 31 + ch.codePointAt(0)) >>> 0;
  }
  return hash;
}

function cross() {
  return new PIXI.Graphics()
    .moveTo(-CROSS_SIZE, -CROSS_SIZE)
    .lineTo(CROSS_SIZE, CROSS_SIZE)
    .moveTo(CROSS_SIZE, -CROSS_SIZE)
    .lineTo(-CROSS_SIZE, CROSS_SIZE)
    .stroke({ color: 0x999999, width: 1.5 });
}

function card({ label, isDir, width, height }, resolution) {
  const c = new PIXI.Container();
  // The position of a card is its center, as stored in atlas_bounds
  c.pivot.set(width / 2, height / 2);
  // A ring outside the frame, so it does not cover the frame's own color
  c.cardSelection = new PIXI.Graphics()
    .roundRect(-3, -3, width + 6, height + 6, 10)
    .stroke({ color: 0x3b82f6, width: 2 });
  c.cardSelection.visible = false;
  c.addChild(c.cardSelection);
  c.cardFrame = new PIXI.Graphics()
    .roundRect(0, 0, width, height, 8)
    .fill(isDir ? 0xfff8e1 : 0xffffff)
    .stroke({ color: isDir ? 0xe0c068 : 0xd0d4da, width: 1 });
  c.addChild(c.cardFrame);
  const text = new PIXI.Text({
    text: label,
    resolution,
    style: {
      fontFamily: getComputedStyle(document.body).fontFamily,
      fontSize: 13,
      fill: 0x222222,
      wordWrap: true,
      wordWrapWidth: Math.max(1, width - 16),
      breakWords: true,
      align: 'center',
    },
  });
  text.anchor.set(0.5);
  text.position.set(width / 2, height / 2);
  // The card handles the press; the label only shows that it opens the page
  text.eventMode = 'static';
  text.cursor = 'pointer';
  c.cardLabel = text;
  // A long name must not spill out of its card
  const mask = new PIXI.Graphics().rect(0, 0, width, height).fill(0xffffff);
  text.mask = mask;
  c.addChild(mask, text);
  return c;
}
