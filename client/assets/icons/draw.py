"""Draws the icons of this folder, as SVG files of 24 by 24 units.

An icon is a set of polygons, each with the corners rounded. They become one filled path:
the outline of the shape, not a line with a width. A line that crosses another would be drawn
twice where they meet, and a faint icon would show that place darker.

A polygon is either a shape or a hole in the shape around it. Shapes go round clockwise, and
holes the other way, so that the path is filled by the nonzero rule.

Change an icon here, then run this file and `export.py`:

    python client/assets/icons/draw.py
    python client/assets/icons/export.py
"""

import math
from pathlib import Path

HERE = Path(__file__).resolve().parent

# How far a corner is cut back to round it, in units. A short side cuts it back less. A point
# given as (x, y, SHARP) keeps its corner: the base of the head of an arrow, and where the head
# meets the body.
ROUND = 0.6
SHARP = 0


def rect(left, top, right, bottom):
    return [(left, top), (right, top), (right, bottom), (left, bottom)]


def arc(cx, cy, r, start, end, steps=24):
    """The points of an arc, from the angle `start` to `end` in degrees, clockwise when end > start."""
    return [
        (cx + r * math.cos(math.radians(start + (end - start) * i / steps)),
         cy + r * math.sin(math.radians(start + (end - start) * i / steps)))
        for i in range(steps + 1)
    ]


def rounded_rect(left, top, right, bottom, r):
    """A rectangle whose corners are quarter circles of the radius `r`."""
    return (arc(right - r, top + r, r, -90, 0, 6) + arc(right - r, bottom - r, r, 0, 90, 6)
            + arc(left + r, bottom - r, r, 90, 180, 6) + arc(left + r, top + r, r, 180, 270, 6))


def stroke(points, half):
    """The outline of a line through the points, `half` units wide on each side, with sharp joins."""
    def offset(p, q, side):
        dx, dy = q[0] - p[0], q[1] - p[1]
        length = math.hypot(dx, dy)
        nx, ny = -dy / length * side * half, dx / length * side * half
        return (p[0] + nx, p[1] + ny), (q[0] + nx, q[1] + ny)

    def meet(a, b):
        (x1, y1), (x2, y2) = a
        (x3, y3), (x4, y4) = b
        d = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4)
        t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / d
        return (x1 + t * (x2 - x1), y1 + t * (y2 - y1))

    sides = []
    for side in (1, -1):
        lines = [offset(points[i], points[i + 1], side) for i in range(len(points) - 1)]
        one = [lines[0][0]] + [meet(lines[i], lines[i + 1]) for i in range(len(lines) - 1)] + [lines[-1][1]]
        sides.append(one)
    return sides[0] + sides[1][::-1]


def turned(points, turn):
    """The points turned about the middle of the icon: 0, 90, 180 or 270 degrees clockwise."""
    for _ in range(turn // 90):
        points = [(24 - y, x) for x, y in points]
    return points


def mirrored(points):
    return [(24 - p[0],) + tuple(p[1:]) for p in points]


def area(points):
    """Twice the area of the polygon, above zero when it goes round clockwise on the screen."""
    return sum(points[i][0] * points[(i + 1) % len(points)][1] - points[(i + 1) % len(points)][0] * points[i][1]
               for i in range(len(points)))


def rounded(points, hole):
    """The path of the polygon with its corners rounded."""
    if (area(points) < 0) != hole:
        points = points[::-1]
    count = len(points)
    cuts = []
    for i in range(count):
        here = points[i][:2]
        before, after = points[i - 1][:2], points[(i + 1) % count][:2]
        wanted = points[i][2] if len(points[i]) > 2 else ROUND
        cut = min(wanted, math.dist(here, before) / 2, math.dist(here, after) / 2)
        if cut == 0:
            cuts.append((here, here, here))
            continue

        def towards(other):
            length = math.dist(here, other)
            return (here[0] + (other[0] - here[0]) * cut / length, here[1] + (other[1] - here[1]) * cut / length)

        cuts.append((towards(before), here, towards(after)))
    f = lambda p: f"{p[0]:.2f} {p[1]:.2f}".replace(".00", "")
    path = f"M{f(cuts[0][2])}"
    for into, corner, out in cuts[1:] + cuts[:1]:
        path += f"L{f(into)}Q{f(corner)} {f(out)}"
    return path + "Z"


def icon(*polygons):
    """An icon from polygons. A polygon is a list of points, or ("hole", points)."""
    parts = []
    for polygon in polygons:
        hole = isinstance(polygon, tuple)
        parts.append(rounded(polygon[1] if hole else polygon, hole))
    return " ".join(parts)


def hole(points):
    return ("hole", points)


# The pads: a triangle at each side

def side_triangle(turn, near=19, far=23, half=3.5):
    return turned([(near, 12 - half), (far, 12), (near, 12 + half)], turn)


def open_triangle(turn):
    outer = turned([(17.5, 7.5), (23, 12), (17.5, 16.5)], turn)
    inner = turned([(18.8, 10.24), (20.95, 12), (18.8, 13.76)], turn)
    # The hole keeps its corners sharp, as the user chose on 2026-11-16
    return [outer, hole([point + (SHARP,) for point in inner])]


def spun(points, degrees, cx, cy):
    """The points turned by any angle about (cx, cy), clockwise on the screen."""
    c, s = math.cos(math.radians(degrees)), math.sin(math.radians(degrees))
    return [(cx + (p[0] - cx) * c - (p[1] - cy) * s, cy + (p[0] - cx) * s + (p[1] - cy) * c) + tuple(p[2:])
            for p in points]


def corner_triangle(cx, cy, degrees, size, open_):
    """A triangle at (cx, cy) that points to `degrees`: 0 is right, 90 is down."""
    tip = [(cx - size * 0.25, cy - size * 0.62), (cx + size * 0.5, cy), (cx - size * 0.25, cy + size * 0.62)]
    outer = spun(tip, degrees, cx, cy)
    if not open_:
        return [outer]
    return [outer, hole([p + (SHARP,) for p in inset(outer, LINE)])]


def corner_chevron(cx, cy, degrees, size):
    """A hollow corner triangle without its base: the two sides that meet at the tip, LINE wide."""
    a, tip, b = corner_triangle(cx, cy, degrees, size, open_=False)[0]
    middle = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)

    def inward(p, q):
        """The line from p to q, moved by LINE towards the middle of the base."""
        dx, dy = q[0] - p[0], q[1] - p[1]
        length = math.hypot(dx, dy)
        nx, ny = -dy / length * LINE, dx / length * LINE
        if (middle[0] - p[0]) * nx + (middle[1] - p[1]) * ny < 0:
            nx, ny = -nx, -ny
        return (p[0] + nx, p[1] + ny), (q[0] + nx, q[1] + ny)

    def meet(one, other):
        (x1, y1), (x2, y2) = one
        (x3, y3), (x4, y4) = other
        d = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4)
        t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / d
        return (x1 + t * (x2 - x1), y1 + t * (y2 - y1))

    side_a, side_b, base = inward(a, tip), inward(b, tip), (a, b)
    inner_tip = meet(side_a, side_b)
    return [a, tip, b, meet(side_b, base) + (SHARP,), inner_tip + (SHARP,), meet(side_a, base) + (SHARP,)]


# How wide the line of a hollow shape is, as in the hollow triangles of the cursor pad
LINE = 1.3


def inset(triangle, width):
    """The triangle with every side moved in by `width`: the hole of a hollow triangle."""
    (ax, ay), (bx, by), (cx, cy) = [p[:2] for p in triangle]
    a, b, c = math.dist((bx, by), (cx, cy)), math.dist((ax, ay), (cx, cy)), math.dist((ax, ay), (bx, by))
    ix, iy = (a * ax + b * bx + c * cx) / (a + b + c), (a * ay + b * by + c * cy) / (a + b + c)
    r = abs(area(triangle)) / (a + b + c)
    k = (r - width) / r
    return [(ix + (x - ix) * k, iy + (y - iy) * k) for x, y in [(ax, ay), (bx, by), (cx, cy)]]


def keyboard(arrow_up):
    keys = [rect(x, y, x + 1.6, y + 1.6) for y in (5.2, 8.2) for x in (5, 8.1, 11.2, 14.3, 17.4)]
    chevron = corner_chevron(12, 20.2, 270, 6.4) if arrow_up else corner_chevron(12, 18.6, 90, 6.4)
    return icon(
        rounded_rect(2, 2, 22, 15, 2),
        hole(rounded_rect(3.3, 3.3, 20.7, 13.7, 0.9)),
        *keys,
        rect(7, 11, 17, 12.2),
        chevron,
    )


UNDO = (
    [(4.5, 7.2), (11, 2.5, SHARP), (11, 6, SHARP)]
    + arc(12, 13, 7, -90, 90)
    + [(8, 20), (8, 17.6)]
    + arc(12, 13, 4.6, 90, -90)
    + [(11, 8.4, SHARP), (11, 11.9, SHARP)]
)

def ring(cx, cy, outer, inner):
    """A ring: a circle with a round hole in it."""
    return [arc(cx, cy, outer, 0, 360, 32)[:-1], hole(arc(cx, cy, inner, 0, 360, 32)[:-1])]


ICONS = {
    "collapse": icon(
        rect(4, 8.5, 20, 10.5),
        rect(4, 13.5, 20, 15.5),
        [(11, 1), (13, 1), (13, 4, SHARP), (16, 4, SHARP), (12, 7.5), (8, 4, SHARP), (11, 4, SHARP)],
        [(12, 16.5), (16, 20, SHARP), (13, 20, SHARP), (13, 23), (11, 23), (11, 20, SHARP), (8, 20, SHARP)],
    ),
    "expand": icon(
        rect(4, 1, 20, 3),
        rect(4, 21, 20, 23),
        [(12, 4.5), (16, 8.5, SHARP), (13, 8.5, SHARP), (13, 15.5, SHARP), (16, 15.5, SHARP), (12, 19.5),
         (8, 15.5, SHARP), (11, 15.5, SHARP), (11, 8.5, SHARP), (8, 8.5, SHARP)],
    ),
    "task": icon(
        rounded_rect(3, 3, 21, 21, 3),
        hole(rounded_rect(5, 5, 19, 19, 1.5)),
        stroke([(8, 12.5), (11, 15.5), (16.5, 9)], 1),
    ),
    "move_block": icon(
        *[side_triangle(turn) for turn in (0, 90, 180, 270)],
        rect(8, 7.7, 16, 9.3),
        rect(8, 11.2, 16, 12.8),
        rect(8, 14.7, 14, 16.3),
    ),
    "move_cursor": icon(
        *[part for turn in (0, 90, 180, 270) for part in open_triangle(turn)],
        # The bar of a cursor, its ends dented a little in the middle
        [(10.6, 7.4), (11.25, 7.4), (12, 7.75), (12.75, 7.4), (13.4, 7.4), (13.4, 8.8), (12.75, 8.8), (12.75, 15.2),
         (13.4, 15.2), (13.4, 16.6), (12.75, 16.6), (12, 16.25), (11.25, 16.6), (10.6, 16.6), (10.6, 15.2),
         (11.25, 15.2), (11.25, 8.8), (10.6, 8.8)],
    ),
    "undo": icon(UNDO),
    "redo": icon(mirrored(UNDO)),
    "hide_keyboard": keyboard(arrow_up=False),
    "show_keyboard": keyboard(arrow_up=True),
    # The pad while items are selected: up and down choose the item above or below
    "move_selection": icon(
        *open_triangle(90),
        *open_triangle(270),
        rounded_rect(6, 10.4, 18, 13.6, 1),
        hole(rounded_rect(7.4, 11.6, 16.6, 12.4, 0.2)),
    ),
    # The selected items go onto the clipboard: a sheet before another
    "copy": icon(
        rounded_rect(9, 7, 20, 21, 2),
        hole(rounded_rect(10.3, 8.3, 18.7, 19.7, 0.9)),
        stroke([(4.65, 17), (4.65, 3.65), (15, 3.65)], 0.65),
    ),
    # The selected items go onto the clipboard and off the page: scissors
    "cut": icon(
        *ring(7, 18, 3.2, 1.9),
        *ring(17, 18, 3.2, 1.9),
        stroke([(8.6, 15.4), (16.5, 2.5)], 0.75),
        stroke([(15.4, 15.4), (7.5, 2.5)], 0.75),
    ),
    # Two triangles point in: the tools become a small button
    "shrink_tools": icon(
        corner_chevron(6.32, 7.74, 36.87, 6.6),
        corner_chevron(17.68, 16.26, 216.87, 6.6),
    ),
    # The face of the small button: two triangles point out, to the full tools
    "expand_tools": icon(
        rounded_rect(1.5, 4, 22.5, 20, 3.5),
        hole(rounded_rect(2.8, 5.3, 21.2, 18.7, 2.2)),
        corner_chevron(6.62, 9.12, 225, 5.8),
        corner_chevron(17.38, 14.88, 45, 5.8),
    ),
}

for name, path in ICONS.items():
    (HERE / f"{name}.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">\n  <path d="{path}"/>\n</svg>\n',
        encoding="utf-8", newline="\n")
