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


UNDO = (
    [(4.5, 7.2), (11, 2.5, SHARP), (11, 6, SHARP)]
    + arc(12, 13, 7, -90, 90)
    + [(8, 20), (8, 17.6)]
    + arc(12, 13, 4.6, 90, -90)
    + [(11, 8.4, SHARP), (11, 11.9, SHARP)]
)

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
    "hide_keyboard": icon(
        rounded_rect(2, 2, 22, 15, 2),
        hole(rounded_rect(3.6, 3.6, 20.4, 13.4, 0.8)),
        *[rect(x, y, x + 1.6, y + 1.6) for y in (5.2, 8.2) for x in (5, 8.1, 11.2, 14.3, 17.4)],
        rect(7, 11, 17, 12.2),
        stroke([(7.4, 17.6), (12, 21.4), (16.6, 17.6)], 0.9),
    ),
}

for name, path in ICONS.items():
    (HERE / f"{name}.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">\n  <path d="{path}"/>\n</svg>\n',
        encoding="utf-8", newline="\n")
