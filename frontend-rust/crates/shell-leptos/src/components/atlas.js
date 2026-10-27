// The canvas of an atlas page: the cards of the pages and folders of its directory, drawn
// with PixiJS. PixiJS is the global `PIXI` of vendor/pixi.min.js. The origin of the card
// coordinates is the center of the canvas, marked by a gray cross. It stays in the center
// when the host is resized, so the cards keep their places around it.
//
// A plain script that sets the global `AtlasCanvas`, loaded by index.html. Not a module
// of wasm-bindgen: Trunk puts a hash of the content in the name of a script it loads, so a
// browser never keeps an old copy. A module keeps the same name when it changes.

const CROSS_SIZE = 8;
// A press that moves less than this is a click, not a drag
const DRAG_THRESHOLD = 3;

window.AtlasCanvas = class AtlasCanvas {
  // onOpen(path, newTab, newWindow) opens the page of a card whose label is clicked.
  // onPlace(pageId, x, y) places the card of a page with its center at a point.
  constructor(host, onOpen, onPlace) {
    this.onOpen = onOpen;
    this.onPlace = onPlace;
    this.drag = null;
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
        this.center();
        this.build();
      });
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
      const view = card(data);
      view.position.set(data.x, data.y);
      this.makeDraggable(view, data);
      this.cards.addChild(view);
    }
  }

  // A drag moves the card and places it where it is let go. A press on the label that
  // does not move opens the page instead.
  makeDraggable(view, data) {
    view.eventMode = 'static';
    view.cursor = 'grab';
    view.on('pointerdown', e => {
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
      view.position.set(drag.origin.x + dx, drag.origin.y + dy);
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

  // Whole pixels: the drop point is measured the same way, and the lines stay sharp
  center() {
    this.world.position.set(
      Math.round(this.app.screen.width / 2),
      Math.round(this.app.screen.height / 2),
    );
  }

  destroy() {
    this.destroyed = true;
    if (this.cards) {
      this.app.destroy({ removeView: true }, { children: true });
    }
  }
};

function cross() {
  return new PIXI.Graphics()
    .moveTo(-CROSS_SIZE, -CROSS_SIZE)
    .lineTo(CROSS_SIZE, CROSS_SIZE)
    .moveTo(CROSS_SIZE, -CROSS_SIZE)
    .lineTo(-CROSS_SIZE, CROSS_SIZE)
    .stroke({ color: 0x999999, width: 1.5 });
}

function card({ label, isDir, width, height }) {
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
