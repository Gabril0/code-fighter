#!/usr/bin/env python3
"""Generate a reproducible preview seed for Code Fighter.

Creates two teams of three fighters whose portraits are original, hand-drawn
SVG avatars evoking famous computer-science figures. No copyrighted photos are
used: every face is built from primitive shapes here, so the seed is safe to
commit and ship in screenshots/tests.

Usage:
    python3 tools/make_preview_seed.py > server/preview-seed.json
"""

import base64
import json
import sys


def face_svg(skin, hair, hair_style, accent, glasses=False,
             mustache=False, beard=False, bald=False):
    """Return an original 256x256 SVG portrait built from primitives."""
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" '
        'viewBox="0 0 256 256">',
        f'<rect width="256" height="256" fill="{accent}"/>',
        # shoulders / torso hint
        f'<rect x="40" y="205" width="176" height="80" rx="42" fill="#1f2937"/>',
        # neck
        f'<rect x="112" y="168" width="32" height="34" fill="{skin}"/>',
        # head
        f'<ellipse cx="128" cy="120" rx="62" ry="70" fill="{skin}"/>',
        # ears
        f'<circle cx="66" cy="122" r="12" fill="{skin}"/>',
        f'<circle cx="190" cy="122" r="12" fill="{skin}"/>',
    ]

    if hair_style == "short":
        parts.append(f'<path d="M66 108 Q128 34 190 108 Q170 78 128 76 '
                     f'Q86 78 66 108 Z" fill="{hair}"/>')
    elif hair_style == "curls":
        parts.append(f'<path d="M60 118 Q52 60 110 54 Q128 40 152 56 '
                     f'Q206 66 196 120 Q186 92 168 92 Q160 70 128 70 '
                     f'Q96 70 88 92 Q70 92 60 118 Z" fill="{hair}"/>')
        for cx in (60, 74, 182, 196):
            parts.append(f'<circle cx="{cx}" cy="140" r="14" fill="{hair}"/>')
    elif hair_style == "long":
        parts.append(f'<path d="M56 92 Q80 40 128 40 Q176 40 200 92 '
                     f'L200 196 Q184 150 178 118 Q160 84 128 82 '
                     f'Q96 84 78 118 Q72 150 56 196 Z" fill="{hair}"/>')
    elif hair_style == "sides":  # bald on top, hair on sides
        parts.append(f'<path d="M64 132 Q60 96 78 92 L82 128 Z" fill="{hair}"/>')
        parts.append(f'<path d="M192 132 Q196 96 178 92 L174 128 Z" fill="{hair}"/>')
        parts.append(f'<path d="M70 120 Q68 96 90 92 Q80 108 82 126 Z" '
                     f'fill="{hair}"/>')
        parts.append(f'<path d="M186 120 Q188 96 166 92 Q176 108 174 126 Z" '
                     f'fill="{hair}"/>')

    # eyes
    parts.append('<ellipse cx="104" cy="120" rx="7" ry="9" fill="#20262e"/>')
    parts.append('<ellipse cx="152" cy="120" rx="7" ry="9" fill="#20262e"/>')
    # brows
    parts.append(f'<rect x="92" y="102" width="26" height="5" rx="2" fill="{hair}"/>')
    parts.append(f'<rect x="138" y="102" width="26" height="5" rx="2" fill="{hair}"/>')
    # nose
    parts.append(f'<path d="M128 124 L120 146 L136 146 Z" fill="{_shade(skin)}"/>')

    if glasses:
        parts.append('<g fill="none" stroke="#111827" stroke-width="5">'
                     '<rect x="88" y="108" width="34" height="26" rx="8"/>'
                     '<rect x="134" y="108" width="34" height="26" rx="8"/>'
                     '<line x1="122" y1="120" x2="134" y2="120"/></g>')

    if beard:
        parts.append(f'<path d="M74 132 Q80 200 128 202 Q176 200 182 132 '
                     f'Q168 178 128 180 Q88 178 74 132 Z" fill="{hair}" '
                     f'opacity="0.92"/>')
        # mouth over beard
        parts.append('<rect x="112" y="160" width="32" height="6" rx="3" fill="#6b2b2b"/>')
    else:
        # smile
        parts.append('<path d="M108 158 Q128 176 148 158" fill="none" '
                     'stroke="#7a3b3b" stroke-width="6" stroke-linecap="round"/>')

    if mustache and not beard:
        parts.append(f'<path d="M108 154 Q128 164 148 154 Q128 172 108 154 Z" '
                     f'fill="{hair}"/>')

    parts.append('</svg>')
    return "".join(parts)


def _shade(hex_color):
    """Slightly darker variant of a skin tone for the nose shadow."""
    r = max(0, int(hex_color[1:3], 16) - 26)
    g = max(0, int(hex_color[3:5], 16) - 26)
    b = max(0, int(hex_color[5:7], 16) - 26)
    return f"#{r:02x}{g:02x}{b:02x}"


def data_url(svg):
    b64 = base64.b64encode(svg.encode("utf-8")).decode("ascii")
    return f"data:image/svg+xml;base64,{b64}"


FIGHTERS = [
    # token, name, team, portrait kwargs
    ("blue-ada", "Player 1", "team_blue",
     dict(skin="#f1c9a5", hair="#3b2417", hair_style="curls", accent="#7cc6ff")),
    ("blue-grace", "Player 2", "team_blue",
     dict(skin="#efc6a8", hair="#c9ced6", hair_style="short", accent="#8fd3ff")),
    ("blue-margaret", "Player 3", "team_blue",
     dict(skin="#eab892", hair="#4a2e1c", hair_style="long", glasses=True, accent="#7fbcf0")),
    ("red-alan", "Player 4", "team_red",
     dict(skin="#f0c7a2", hair="#5a3a24", hair_style="short", accent="#ff9a9a")),
    ("red-dennis", "Player 5", "team_red",
     dict(skin="#e6b58c", hair="#6b4a2f", hair_style="short", glasses=True, beard=True, accent="#ff8f8f")),
    ("red-donald", "Player 6", "team_red",
     dict(skin="#f0cbad", hair="#e8ebef", hair_style="sides", glasses=True, beard=True, bald=True, accent="#ffb0b0")),
]

# Crowd in the stands. Original SVG faces, generic names, one per seat.
SPECTATORS = [
    ("Spectator 1", dict(skin="#f1c9a5", hair="#2f2a24", hair_style="short", accent="#bfe6fa")),
    ("Spectator 2", dict(skin="#e3ac82", hair="#3a2a1c", hair_style="curls", accent="#cdefff")),
    ("Spectator 3", dict(skin="#f0cbad", hair="#5a3a24", hair_style="long", accent="#d7f0ff")),
    ("Spectator 4", dict(skin="#d99c6e", hair="#26221d", hair_style="short", glasses=True, accent="#bfe6fa")),
    ("Spectator 5", dict(skin="#efc6a8", hair="#c9ced6", hair_style="sides", beard=True, accent="#cdefff")),
    ("Spectator 6", dict(skin="#eab892", hair="#4a2e1c", hair_style="curls", accent="#d7f0ff")),
    ("Spectator 7", dict(skin="#f0c7a2", hair="#6b4a2f", hair_style="short", mustache=True, accent="#bfe6fa")),
    ("Spectator 8", dict(skin="#c98a5e", hair="#201a16", hair_style="long", accent="#cdefff")),
    ("Spectator 9", dict(skin="#f1c9a5", hair="#7a5a3a", hair_style="short", glasses=True, accent="#d7f0ff")),
    ("Spectator 10", dict(skin="#e6b58c", hair="#3b2417", hair_style="curls", beard=True, accent="#bfe6fa")),
    ("Spectator 11", dict(skin="#efc6a8", hair="#4a2e1c", hair_style="sides", accent="#cdefff")),
    ("Spectator 12", dict(skin="#eab892", hair="#5a3a24", hair_style="short", glasses=True, accent="#d7f0ff")),
]


def main():
    users = []
    for idx, (token, name, team, kw) in enumerate(FIGHTERS, start=1):
        users.append({
            "id": f"u{idx}",
            "token": token,
            "name": name,
            "role": "participant",
            "team_id": team,
            "profile_set": True,
            "photo": data_url(face_svg(**kw)),
        })

    spectators = []
    for seat, (name, kw) in enumerate(SPECTATORS):
        spectators.append({
            "id": f"spec{seat + 1}",
            "token": f"watch-seed-{seat + 1}",
            "name": name,
            "seat": seat,
            "photo": data_url(face_svg(**kw)),
        })

    seed = {
        "teams": [
            {"id": "team_blue", "name": "Blue Corner", "color": "#3b82f6"},
            {"id": "team_red", "name": "Red Corner", "color": "#ef4444"},
        ],
        "users": users,
        "spectators": spectators,
    }
    json.dump(seed, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
