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

window.AtlasCanvas = class AtlasCanvas {
  // onOpen(path, newTab, newWindow) opens the page of a card whose label is clicked.
  // onPlace(pageId, x, y) places the card of a page with its center at a point.
  // onMenu(pageId, clientX, clientY) opens the menu of a card that is right-clicked.
  constructor(host, onOpen, onPlace, onMenu) {
    this.onOpen = onOpen;
    this.onPlace = onPlace;
    this.onMenu = onMenu;
    this.drag = null;
    // How far the hand tool has moved the view from the center of the canvas
    this.pan = { x: 0, y: 0 };
    this.zoom = 1;
    this.panning = null;
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
        this.world.addChild(cross(), this.cards);
        this.app.stage.addChild(this.world);
        this.app.renderer.on('resize', () => this.center());
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
    // Drawing every label again on each step of the wheel would make the zoom stutter
    clearTimeout(this.sharpenTimer);
    this.sharpenTimer = setTimeout(() => this.sharpenText(), SHARPEN_DELAY);
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
    if (!panning.moved && panning.button === 2 && panning.card) {
      this.onMenu(panning.card.cardData.id, e.clientX, e.clientY);
    }
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
  }

  // A drag moves the card and places it where it is let go. A press on the label that
  // does not move opens the page instead.
  makeDraggable(view, data) {
    view.cardData = data;
    view.eventMode = 'static';
    view.cursor = 'grab';
    view.on('pointerdown', e => {
      // The right button is the hand tool; only the left button drags a card
      if (e.button !== 0) {
        return;
      }
      this.drag = {
        view,
        // Plain copies: `global` is reused for the next event
        start: { x: e.global.x, y: e.global.y },
        origin: { x: view.x, y: view.y },
        moved: false,
        onLabel: e.target === view.cardLabel,
      };
      // On top of the others while it moves
      this.cards.addChild(view);
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
      // The pointer moves in pixels of the screen, the card in units of the scaled view
      view.position.set(drag.origin.x + dx / this.zoom, drag.origin.y + dy / this.zoom);
    });
    const end = e => {
      const drag = this.drag;
      if (!drag || drag.view !== view) {
        return;
      }
      this.drag = null;
      view.cursor = 'grab';
      if (drag.moved) {
        // The card has moved with the pointer from where it was grabbed: it lies where it shows.
        // Its position is its center, as the pivot is there.
        this.onPlace(data.id, view.x, view.y);
      } else if (drag.onLabel) {
        this.onOpen(data.path, e.ctrlKey || e.metaKey, e.shiftKey);
      }
    };
    view.on('pointerup', end);
    view.on('pointerupoutside', end);
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
  c.addChild(
    new PIXI.Graphics()
      .roundRect(0, 0, width, height, 8)
      .fill(isDir ? 0xfff8e1 : 0xffffff)
      .stroke({ color: isDir ? 0xe0c068 : 0xd0d4da, width: 1 }),
  );
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
