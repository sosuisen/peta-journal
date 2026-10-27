// The canvas of an atlas page: a card for each page and folder of the directory, drawn with
// PixiJS. PixiJS is the global `PIXI` of vendor/pixi.min.js. The canvas fills its host and
// lays the cards out again when the host is resized.

const CARD_WIDTH = 160;
const CARD_HEIGHT = 56;
const GAP = 16;
const MARGIN = 24;

export class AtlasCanvas {
  constructor(host) {
    this.labels = [];
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
        this.cards = new PIXI.Container();
        this.app.stage.addChild(this.cards);
        this.app.renderer.on('resize', () => this.layout());
        this.build();
      });
  }

  setCards(labels) {
    this.labels = labels;
    if (this.cards) {
      this.build();
    }
  }

  build() {
    this.cards.removeChildren().forEach((c) => c.destroy({ children: true }));
    for (const label of this.labels) {
      this.cards.addChild(card(label));
    }
    this.layout();
  }

  layout() {
    const width = this.app.screen.width;
    const columns = Math.max(1, Math.floor((width - 2 * MARGIN + GAP) / (CARD_WIDTH + GAP)));
    this.cards.children.forEach((c, i) => {
      c.position.set(
        MARGIN + (i % columns) * (CARD_WIDTH + GAP),
        MARGIN + Math.floor(i / columns) * (CARD_HEIGHT + GAP),
      );
    });
  }

  destroy() {
    this.destroyed = true;
    if (this.cards) {
      this.app.destroy({ removeView: true }, { children: true });
    }
  }
}

function card(label) {
  const c = new PIXI.Container();
  const isFolder = label.endsWith('/');
  c.addChild(
    new PIXI.Graphics()
      .roundRect(0, 0, CARD_WIDTH, CARD_HEIGHT, 8)
      .fill(isFolder ? 0xfff8e1 : 0xffffff)
      .stroke({ color: isFolder ? 0xe0c068 : 0xd0d4da, width: 1 }),
  );
  const text = new PIXI.Text({
    text: label,
    style: {
      fontFamily: getComputedStyle(document.body).fontFamily,
      fontSize: 14,
      fill: 0x222222,
      wordWrap: true,
      wordWrapWidth: CARD_WIDTH - 24,
      breakWords: true,
      align: 'center',
    },
  });
  text.anchor.set(0.5);
  text.position.set(CARD_WIDTH / 2, CARD_HEIGHT / 2);
  // A long name must not spill out of its card
  const mask = new PIXI.Graphics().rect(0, 0, CARD_WIDTH, CARD_HEIGHT).fill(0xffffff);
  text.mask = mask;
  c.addChild(mask, text);
  return c;
}
