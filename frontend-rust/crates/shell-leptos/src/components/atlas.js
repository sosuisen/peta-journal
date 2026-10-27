// The canvas of an atlas page: the cards of the pages and folders of its directory, drawn
// with PixiJS. PixiJS is the global `PIXI` of vendor/pixi.min.js. The origin of the card
// coordinates is the center of the canvas, marked by a gray cross. It stays in the center
// when the host is resized, so the cards keep their places around it.
//
// The hand tool moves the view: a drag with the right button, or with the left one
// between the cards and the groups. It moves the container of the cards and the cross, not
// the cards. A drag with the left button on the area of a group moves all its cards.
// The wheel scales that container around the point under the pointer. With Ctrl, it scales
// the selected cards instead, each around its own center.
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
        this.groups = new PIXI.Container();
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
    const delta = e.deltaMode === WheelEvent.DOM_DELTA_PIXEL ? e.deltaY : e.deltaY * LINE_HEIGHT;
    if ((e.ctrlKey || e.metaKey) && this.selected.size > 0) {
      this.scaleSelected(Math.exp(-delta * ZOOM_PER_PIXEL));
      return;
    }
    const rect = this.app.canvas.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
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

  // The selected cards grow or shrink by the factor, each around its center, as the pivot is
  // there. Only the outlines of their groups are drawn again. The scales are saved once the
  // wheel rests, as each save gives a new list of cards.
  scaleSelected(factor) {
    const views = this.cards.children.filter(v => this.selected.has(v.cardData.id));
    for (const view of views) {
      const scale = Math.max(MIN_CARD_SCALE, view.scale.x * factor);
      view.scale.set(scale);
      view.cardData.width = view.cardWidth * scale;
      view.cardData.height = view.cardHeight * scale;
    }
    // Not a drag: the outlines of the groups of these cards only, with no frame shown
    this.drawGroups({ origins: views.map(v => ({ view: v, x: v.x, y: v.y })), moved: false });
    clearTimeout(this.scaleTimer);
    this.scaleTimer = setTimeout(() => {
      this.sharpenText();
      if (this.onScale) {
        for (const view of views) {
          this.onScale(view.cardData.id, view.scale.x);
        }
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
    for (const view of this.cards.children) {
      const resolution = this.textResolution(view.scale.x);
      if (view.cardLabel.resolution !== resolution) {
        view.cardLabel.resolution = resolution;
      }
    }
  }

  startPan(e) {
    // The left button on a card drags the card, on a group its cards; elsewhere it moves the
    // view
    const card = cardOf(e.target);
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
      // A right click that does not move opens the menu of this card
      card,
    };
    this.showHand();
  }

  movePan(e) {
    if (this.groupDrag) {
      this.moveGroupDrag(e);
      return;
    }
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
    if (this.groupDrag && e.button === 0) {
      this.endGroupDrag();
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
    } else if (panning.button === 2 && panning.card) {
      this.onMenu(panning.card.cardData.id, e.clientX, e.clientY);
    } else if (panning.button === 0 && !panning.card) {
      // A click between the cards lets go of the selection
      this.select([]);
    }
  }

  // A group is dragged as its cards are, without Ctrl: it keeps its cards and joins no other
  startGroupDrag(e, area) {
    const views = this.cards.children.filter(v => v.cardData.group === area.label);
    this.groupDrag = {
      start: { x: e.global.x, y: e.global.y },
      origins: views.map(v => ({ view: v, x: v.x, y: v.y })),
      moved: false,
    };
    // Nothing else takes the press or shows its cursor while the group moves
    this.cards.interactiveChildren = false;
    this.groups.interactiveChildren = false;
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
    for (const o of drag.origins) {
      o.view.position.set(o.x + dx / this.zoom, o.y + dy / this.zoom);
    }
    this.drawGroups(drag);
  }

  endGroupDrag() {
    const drag = this.groupDrag;
    this.groupDrag = null;
    this.cards.interactiveChildren = true;
    this.groups.interactiveChildren = true;
    this.app.stage.cursor = 'default';
    if (!drag.moved) {
      // A click on a group, as between the cards, lets go of the selection
      this.select([]);
      return;
    }
    this.showFrames(null);
    // Read before placing any: placing one gives a new list of cards
    const places = drag.origins.map(o => [o.view.cardData.id, o.view.x, o.view.y]);
    for (const [id, x, y] of places) {
      this.onPlace(id, x, y);
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
    this.groups.interactiveChildren = this.panning === null;
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

  // onUngroup(pageIdsJson) takes the cards let go outside their group with Ctrl out of it
  setOnUngroup(onUngroup) {
    this.onUngroup = onUngroup;
  }

  // onScale(pageId, scale) scales the card of a page
  setOnScale(onScale) {
    this.onScale = onScale;
  }

  // The placed cards as JSON: [{ id, label, path, isDir, x, y, width, height, scale, group,
  // groupColor, icon, blob, iconSize, iconDrawSize }], where x and y are the center of the card,
  // width and height are its size as scaled, a width of 0 leaves it to the label, icon and blob are SVGs or null, and iconDrawSize is the size of the SVG, which is shown at iconSize. The core gives the color of
  // each group, so it is the same as on the tag icons.
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
      // The icons of a card scaled up are drawn in more pixels too
      const view = card(data, this.textResolution(data.scale), this.iconResolution() * Math.max(1, data.scale));
      view.position.set(data.x, data.y);
      this.makeDraggable(view, data);
      this.cards.addChild(view);
    }
    // A card taken off the canvas is no longer selected
    this.selected = new Set(this.data.map(d => d.id).filter(id => this.selected.has(id)));
    this.showSelection();
    this.drawGroups();
  }

  // Each group is a smooth closed curve around all its cards, one Graphics for its area and one
  // for its frame. The frame is drawn in white and colored by its tint, so a change of its
  // color needs no new geometry. Given a drag, only the groups of the dragged cards are drawn again. With `ctrl`, the
  // outline of such a group stays as it was before the drag, with the dragged cards where
  // they were, so it shows whether they are let go in it. That shape does not change, so it
  // is drawn once while Ctrl is held; only a group dragged as a whole moves with its cards.
  drawGroups(drag = null, ctrl = false) {
    const origins = new Map(drag ? drag.origins.map(o => [o.view, o]) : []);
    const dragged = new Set(origins.keys());
    const touched = new Set([...dragged].map(v => v.cardData.group).filter(Boolean));
    const staying = new Set();
    for (const view of this.cards.children) {
      if (view.cardData.group && !dragged.has(view)) {
        staying.add(view.cardData.group);
      }
    }
    const corners = new Map();
    const colors = new Map();
    for (const view of this.cards.children) {
      const { group, width, height } = view.cardData;
      if (!group || (drag && !touched.has(group))) {
        continue;
      }
      const kept = ctrl && staying.has(group);
      if (kept && drag.ctrlShape) {
        continue;
      }
      const { x, y } = kept && dragged.has(view) ? origins.get(view) : view;
      const left = x - width / 2 - GROUP_PADDING;
      const right = x + width / 2 + GROUP_PADDING;
      const top = y - height / 2 - GROUP_PADDING;
      const bottom = y + height / 2 + GROUP_PADDING;
      if (!corners.has(group)) {
        corners.set(group, []);
        colors.set(group, view.cardData.groupColor);
      }
      corners.get(group).push([left, top], [right, top], [right, bottom], [left, bottom]);
    }
    if (!drag) {
      this.groups.removeChildren().forEach(g => g.destroy());
    }
    for (const [group, points] of corners) {
      let area = this.groups.children.find(c => c.label === group);
      if (!area) {
        area = new PIXI.Graphics({ label: group });
        // A press on the area drags the group. Not on its frame, which only shows the edge.
        area.eventMode = 'static';
        area.cursor = 'grab';
        area.groupArea = true;
        area.frame = new PIXI.Graphics();
        // The frame shows only while a card of the group is dragged
        area.frame.visible = Boolean(drag && drag.moved);
        this.groups.addChild(area, area.frame);
      }
      const color = colors.get(group);
      // One path for the area and its frame
      const curve = new PIXI.GraphicsPath();
      smoothClosedCurve(curve, convexHull(points));
      area.clear().path(curve).fill({ color, alpha: 0.3 });
      area.frame.clear().path(curve).stroke({ color: 0xffffff, width: 2 });
      area.frame.tint = color;
      // Not `groupColor`: a Container uses that name for the tint it passes to its children,
      // and a color written there turns the area black while it is drawn again
      area.outlineColor = color;
      // A group dragged as a whole has nothing to leave: it moves with its cards
      area.leavable = ctrl && staying.has(group);
    }
  }

  // The frame of a group shows while a card of it is dragged, so it is clear where the group
  // ends; at rest, and for the other groups, only the areas show. No drag hides every frame.
  showFrames(drag) {
    const dragged = new Set(drag ? drag.origins.map(o => o.view.cardData.group).filter(Boolean) : []);
    for (const area of this.groups.children) {
      if (area.frame) {
        area.frame.visible = dragged.has(area.label);
      }
    }
  }

  // While cards are dragged with Ctrl, the frame of a group they can leave takes the color of
  // a held group as long as the pointer is in its area, and its own color once they would
  // leave it. The area keeps its color. The groups they would leave are kept in the drag for
  // the drop. A pointer in the area of another group takes the cards out of their own, as
  // they join that group.
  showHeldGroups(drag, e) {
    drag.outside = new Set();
    for (const area of this.groups.children) {
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
  // group is to be joined. The topmost area wins, as it is drawn over the others.
  showJoinGroup(drag, e) {
    const own = new Set(drag.origins.map(o => o.view.cardData.group).filter(Boolean));
    let join = null;
    if (e) {
      const areas = this.groups.children.filter(c => c.frame && !own.has(c.label));
      for (let i = areas.length - 1; i >= 0 && !join; i--) {
        if (areas[i].containsPoint(areas[i].toLocal(e.global))) {
          join = areas[i];
        }
      }
    }
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
      if (!drag.moved) {
        this.showFrames(drag);
      }
      drag.moved = true;
      view.cursor = 'grabbing';
      // The pointer moves in pixels of the screen, the cards in units of the scaled view
      for (const o of drag.origins) {
        o.view.position.set(o.x + dx / this.zoom, o.y + dy / this.zoom);
      }
      const ctrl = e.ctrlKey || e.metaKey;
      this.drawGroups(drag, ctrl);
      drag.ctrlShape = ctrl;
      if (ctrl) {
        this.showJoinGroup(drag, e);
        this.showHeldGroups(drag, e);
      } else {
        this.showJoinGroup(drag, null);
        drag.outside = null;
      }
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
        this.showFrames(null);
        const target = this.groupTargetAt(drag, e);
        this.showGroupTarget(drag, null);
        // The cards have moved with the pointer: they lie where they show. A position is the
        // center of a card, as the pivot is there. Read before placing any: placing one
        // gives a new list of cards, which destroys these views.
        const places = drag.origins.map(o => [o.view.cardData.id, o.view.x, o.view.y]);
        const ctrl = e.ctrlKey || e.metaKey;
        // A card let go on a card joins the group of that card; let go in the area of another
        // group, it joins that group, which any card of it stands for
        const joined = ctrl && drag.join && this.cards.children.find(v => v.cardData.group === drag.join.label);
        const targetId = target ? target.cardData.id : joined ? joined.cardData.id : null;
        // Joining a group takes the cards out of their own; only the cards let go outside their
        // group without joining another leave it
        const leavers =
          !targetId && ctrl && drag.outside
            ? drag.origins.map(o => o.view.cardData).filter(d => drag.outside.has(d.group)).map(d => d.id)
            : [];
        for (const [id, x, y] of places) {
          this.onPlace(id, x, y);
        }
        if (targetId && this.onGroup) {
          this.onGroup(JSON.stringify(places.map(([id]) => id)), targetId);
        }
        if (leavers.length > 0 && this.onUngroup) {
          this.onUngroup(JSON.stringify(leavers));
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
    clearTimeout(this.sharpenTimer);
    clearTimeout(this.scaleTimer);
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

// A closed curve through every point, drawn on a Graphics or a GraphicsPath, as Bézier curves of a Catmull-Rom spline. It passes
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
function card(data, resolution, iconResolution) {
  const { label, icon, blob, iconSize, iconDrawSize } = data;
  const scale = data.scale > 0 ? data.scale : 1;
  const height = data.height / scale;
  const setWidth = data.width > 0 ? data.width / scale : null;
  const text = new PIXI.Text({
    text: label,
    resolution,
    style: {
      fontFamily: getComputedStyle(document.body).fontFamily,
      fontSize: 13,
      fill: 0x222222,
      // A white edge keeps the label readable over the icon and the groups behind it. The
      // padding keeps the edge from being cut off at the sides of the text.
      stroke: { color: 0xffffff, width: 3, join: 'round' },
      padding: 2,
      wordWrap: true,
      wordWrapWidth: Math.max(1, (setWidth ?? CARD_MAX_WIDTH) - 2 * CARD_PADDING),
      breakWords: true,
      align: 'center',
    },
  });
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
    // Stretched over the whole card, behind the icon. It is a plain fill, so the resolution of
    // the wider side is enough.
    const blobResolution = (iconResolution * Math.max(width, height)) / iconDrawSize;
    inside.addChild(iconSprite(blob, width, height, blobResolution, width / 2, height / 2, 1));
  }
  if (icon) {
    // Rasterized in as many pixels as the icon covers at its largest, not at the size of the SVG
    const rasterResolution = (iconResolution * iconSize) / iconDrawSize;
    // Behind the label, in the middle of the card. Faint, so it stays behind the label.
    inside.addChild(
      iconSprite(icon, iconSize, iconSize, rasterResolution, width / 2, height / 2, ICON_ALPHA),
    );
  }
  text.anchor.set(0.5);
  text.position.set(width / 2, height / 2);
  // The card handles the press; the label only shows that it opens the page
  text.eventMode = 'static';
  text.cursor = 'pointer';
  c.cardLabel = text;
  inside.addChild(text);
  return c;
}
