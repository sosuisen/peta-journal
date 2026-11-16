"""Copies the icons of this folder into the apps that draw them.

Each icon is drawn once, here, as an SVG of 24 by 24 units with one filled path. The path is
the outline of the shape, not a line with a width. A line that crosses another would be drawn
twice where they meet, and a faint icon would show that place darker.

- Android: a Vector Drawable, `res/drawable/ic_<name>.xml`. Android does not read SVG.
- iOS: an image set of the asset catalog, `<name>.imageset`, with the SVG in it. It is drawn
  in the colour of the text around it, as a template.

Run it after a change of an icon, from any folder:

    python client/assets/icons/export.py
"""

import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
APPS = HERE.parent.parent / "apps" / "native-app"
ANDROID = APPS / "outliner-android" / "app" / "src" / "main" / "res" / "drawable"
IOS = APPS / "outliner-ios" / "App" / "Assets.xcassets"

ANDROID_XML = """<?xml version="1.0" encoding="utf-8"?>
<!-- Made by client/assets/icons/export.py from {name}.svg. Change the SVG, not this file. -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="24dp"
    android:height="24dp"
    android:viewportWidth="24"
    android:viewportHeight="24">
    <path
        android:fillColor="#FF000000"
        android:pathData="{path}" />
</vector>
"""

IOS_CONTENTS = {
    "images": [{"filename": None, "idiom": "universal"}],
    "info": {"author": "xcode", "version": 1},
    "properties": {"preserves-vector-representation": True, "template-rendering-intent": "template"},
}


def path_of(svg: Path) -> str:
    paths = re.findall(r'<path d="([^"]+)"', svg.read_text(encoding="utf-8"))
    if len(paths) != 1:
        raise SystemExit(f"{svg.name}: an icon has one path, but it has {len(paths)}")
    return paths[0]


def main() -> None:
    for svg in sorted(HERE.glob("*.svg")):
        name = svg.stem
        (ANDROID / f"ic_{name}.xml").write_text(
            ANDROID_XML.format(name=name, path=path_of(svg)), encoding="utf-8", newline="\n")
        images = IOS / f"{name}.imageset"
        images.mkdir(exist_ok=True)
        (images / svg.name).write_bytes(svg.read_bytes())
        contents = json.loads(json.dumps(IOS_CONTENTS))
        contents["images"][0]["filename"] = svg.name
        (images / "Contents.json").write_text(json.dumps(contents, indent=2) + "\n", encoding="utf-8", newline="\n")


main()
