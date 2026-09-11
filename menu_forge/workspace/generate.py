#!/usr/bin/env python3
"""Espace de travail Menu Forge des écrans de /menu (mc-rs).

Génère les textures (Pillow, dans l'esprit du pack mcrs_ui : bleu nuit,
liserés dorés ; aucun asset tiers) et, sur demande, les menus qui recréent les
écrans de /menu (hub et panneaux de démonstration), plus le menu « demo »
réservé à /mf open demo.

Usage, depuis ce dossier :

    python generate.py            textures seulement (textures/mcrs/)
    python generate.py --menus    textures et menus/*.menu.json (écrase les menus)

Ensuite : ouvrir ce dossier comme espace de travail dans le studio Menu Forge,
puis « Exporter pour Bedrock » vers ../export (voir ../README.md).
"""

import json
import math
import os
import sys

from PIL import Image, ImageDraw

NBSP = chr(160)
HERE = os.path.dirname(os.path.abspath(__file__))
TEXTURES = os.path.join(HERE, "textures", "mcrs")
MENUS = os.path.join(HERE, "menus")

# ── Palette mcrs_ui (generate_assets.py, _global_variables.json) ─────────────
PANEL = (12, 12, 25, 255)
TITLE_BAND = (20, 20, 40, 255)
GOLD = (255, 217, 51, 255)
ORANGE = (255, 154, 30, 255)
BUTTON = (30, 30, 46, 255)
BUTTON_LIGHT = (45, 45, 68, 255)
BORDER = (60, 60, 80, 255)

# Accents des boutons (proches des codes § des anciens formulaires).
ACCENTS = {
    "green": (85, 255, 85),
    "aqua": (85, 255, 255),
    "yellow": (255, 255, 85),
    "pink": (255, 85, 255),
    "blue": (85, 85, 255),
    "gray": (170, 170, 170),
    "gold": (255, 217, 51),
    "red": (255, 85, 85),
}

WIDTH = 176
CELL = 18


def save(image, name):
    path = os.path.join(TEXTURES, f"{name}.png")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    image.save(path, "PNG")


def mix(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3)) + (255,)


def gradient(width, height, top, bottom):
    image = Image.new("RGBA", (width, height))
    draw = ImageDraw.Draw(image)
    for y in range(height):
        draw.line([(0, y), (width - 1, y)], fill=mix(top, bottom, y / max(1, height - 1)))
    return image


def frame(image, color=GOLD):
    draw = ImageDraw.Draw(image)
    draw.rectangle([0, 0, image.width - 1, image.height - 1], outline=color)
    return image


def panel(rows):
    """Fond d'un menu : panneau bleu nuit, bandeau de titre, liseré doré."""
    height = 17 + CELL * rows + 7
    image = Image.new("RGBA", (WIDTH, height), PANEL)
    draw = ImageDraw.Draw(image)
    draw.rectangle([1, 1, WIDTH - 2, 15], fill=TITLE_BAND)
    draw.line([(2, 15), (WIDTH - 3, 15)], fill=GOLD[:3] + (170,))
    draw.rectangle([0, 0, WIDTH - 1, height - 1], outline=GOLD)
    draw.rectangle([1, 1, WIDTH - 2, height - 2], outline=ORANGE[:3] + (90,))
    return image


def button(cells, accent, active=False):
    """Bouton de `cells` cases : fond sombre, bordure, barre d'accent à gauche."""
    width = CELL * cells
    image = Image.new("RGBA", (width, CELL), BUTTON_LIGHT if active else BUTTON)
    draw = ImageDraw.Draw(image)
    draw.line([(1, 1), (width - 2, 1)], fill=BUTTON_LIGHT if not active else GOLD[:3] + (120,))
    draw.rectangle([0, 0, width - 1, CELL - 1], outline=GOLD if active else BORDER)
    draw.rectangle([1, 2, 2, CELL - 3], fill=ACCENTS[accent] + (255,))
    return image


def landscape(kind, width, height):
    """Petites vues dessinées (sélecteur de carte)."""
    skies = {
        "forest": ((110, 170, 255), (190, 225, 255)),
        "desert": ((255, 190, 120), (255, 230, 170)),
        "mountain": ((90, 130, 210), (170, 200, 240)),
        "swamp": ((70, 90, 80), (120, 140, 110)),
    }
    image = gradient(width, height, *skies[kind])
    draw = ImageDraw.Draw(image)
    ground = height * 2 // 3
    if kind == "forest":
        draw.rectangle([0, ground, width, height], fill=(60, 140, 60, 255))
        for x in range(4, width, 11):
            draw.polygon([(x, ground + 2), (x + 5, ground - 12), (x + 10, ground + 2)], fill=(30, 100, 40, 255))
    elif kind == "desert":
        draw.ellipse([width - 18, 4, width - 6, 16], fill=(255, 250, 200, 255))
        for x in range(-10, width, 24):
            draw.ellipse([x, ground - 6, x + 36, ground + 14], fill=(230, 195, 115, 255))
        draw.rectangle([0, ground + 4, width, height], fill=(225, 190, 110, 255))
    elif kind == "mountain":
        draw.rectangle([0, ground, width, height], fill=(90, 110, 90, 255))
        for x, peak in ((0, 8), (24, 4), (46, 10)):
            draw.polygon([(x, ground + 2), (x + 16, peak), (x + 32, ground + 2)], fill=(120, 120, 135, 255))
            draw.polygon([(x + 11, peak + 5), (x + 16, peak), (x + 21, peak + 5)], fill=(245, 245, 250, 255))
    elif kind == "swamp":
        draw.rectangle([0, ground, width, height], fill=(50, 90, 70, 255))
        draw.rectangle([0, ground + 6, width, height], fill=(40, 70, 75, 255))
        for x in range(3, width, 7):
            draw.line([(x, ground + 8), (x + 1, ground - 4)], fill=(110, 140, 60, 255))
    return frame(image)


def banner(width, height, top, bottom, emblem=None):
    image = gradient(width, height, top, bottom)
    draw = ImageDraw.Draw(image)
    for index in range(0, width, 14):
        draw.point((index + 3, 3 + (index * 7) % max(1, height - 6)), fill=(255, 255, 255, 200))
    if emblem == "bed":
        cx, cy = width // 2, height // 2 + 6
        draw.rectangle([cx - 22, cy - 4, cx + 22, cy + 6], fill=(200, 40, 50, 255))
        draw.rectangle([cx - 22, cy - 8, cx - 12, cy - 2], fill=(240, 240, 240, 255))
        draw.rectangle([cx - 24, cy + 6, cx - 21, cy + 10], fill=(90, 60, 30, 255))
        draw.rectangle([cx + 21, cy + 6, cx + 24, cy + 10], fill=(90, 60, 30, 255))
    return frame(image)


def rings(size):
    """Image carrée : anneaux dorés sur fond bleu nuit."""
    image = Image.new("RGBA", (size, size), PANEL)
    draw = ImageDraw.Draw(image)
    center = size / 2
    for radius, color in ((40, GOLD), (32, ORANGE), (24, GOLD), (14, ORANGE), (6, GOLD)):
        draw.ellipse([center - radius, center - radius, center + radius, center + radius], outline=color, width=2)
    for angle in range(0, 360, 45):
        rad = math.radians(angle)
        draw.line(
            [(center + 8 * math.cos(rad), center + 8 * math.sin(rad)), (center + 42 * math.cos(rad), center + 42 * math.sin(rad))],
            fill=GOLD[:3] + (110,),
        )
    return frame(image)


def pictogram(kind):
    """Icône 16 × 16 d'un article de la boutique."""
    image = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    if kind == "sword":
        draw.line([(3, 12), (12, 3)], fill=(220, 230, 255, 255), width=2)
        draw.line([(3, 9), (6, 12)], fill=(160, 110, 40, 255), width=2)
    elif kind == "pickaxe":
        draw.line([(4, 12), (11, 5)], fill=(160, 110, 40, 255), width=2)
        draw.arc([2, 1, 14, 11], 200, 340, fill=(90, 230, 230, 255), width=2)
    elif kind == "skin":
        draw.rectangle([4, 2, 11, 7], fill=(230, 180, 140, 255))
        draw.rectangle([4, 2, 11, 3], fill=(40, 30, 20, 255))
        draw.rectangle([3, 8, 12, 14], fill=(170, 40, 40, 255))
    elif kind == "cape":
        draw.polygon([(3, 2), (12, 2), (14, 14), (1, 14)], fill=(120, 40, 170, 255))
        draw.line([(3, 2), (12, 2)], fill=GOLD, width=2)
    return image


def card(accent, kind):
    """Article de la boutique : bouton pleine largeur et pictogramme."""
    image = button(9, accent)
    image.alpha_composite(pictogram(kind), (4, 1))
    return image


def tab(active):
    width = CELL * 3
    image = Image.new("RGBA", (width, CELL), BUTTON_LIGHT if active else BUTTON)
    draw = ImageDraw.Draw(image)
    draw.rectangle([0, 0, width - 1, CELL - 1], outline=GOLD if active else BORDER)
    if active:
        draw.line([(1, CELL - 2), (width - 2, CELL - 2)], fill=GOLD, width=2)
    return image


def description(width, height):
    image = Image.new("RGBA", (width, height), (22, 22, 38, 255))
    return frame(image, BORDER)


def badge():
    image = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    points = []
    for index in range(10):
        radius = 7 if index % 2 == 0 else 3
        angle = math.radians(-90 + index * 36)
        points.append((7.5 + radius * math.cos(angle), 7.5 + radius * math.sin(angle)))
    draw.polygon(points, fill=GOLD, outline=ORANGE)
    return image


def generate_textures():
    for rows in (4, 5, 6):
        save(panel(rows), f"panel_r{rows}")
    for cells in (1, 3, 4, 5, 9):
        for accent in ACCENTS:
            save(button(cells, accent), f"button_w{cells}_{accent}")
    save(button(4, "gold", active=True), "button_w4_active")
    for kind in ("forest", "desert", "mountain", "swamp"):
        save(landscape(kind, 72, 36), f"map_{kind}")
    save(banner(162, 54, (120, 20, 30), (60, 10, 70), emblem="bed"), "banner_bedwars")
    save(banner(162, 36, (20, 30, 80), (80, 30, 110)), "banner_motd")
    save(rings(90), "square_image")
    for index, colors in enumerate((((255, 154, 30), (120, 40, 10)), ((85, 255, 255), (20, 60, 120)), ((255, 85, 255), (70, 20, 90))), 1):
        save(banner(54, 54, *colors), f"wrapped_{index}")
    for accent, kind in (("gold", "sword"), ("aqua", "pickaxe"), ("red", "skin"), ("pink", "cape")):
        save(card(accent, kind), f"store_{kind}")
    save(tab(True), "tab_on")
    save(tab(False), "tab_off")
    save(description(72, 90), "description")
    save(badge(), "badge")
    print(f"Textures écrites dans {TEXTURES}")


# ── Menus ────────────────────────────────────────────────────────────────────

TEXT = {
    "title": "#ffd933",
    "light": "#f0f0f0",
    "muted": "#aaaaaa",
    "green": "#55ff55",
    "aqua": "#55ffff",
    "yellow": "#ffff55",
    "pink": "#ff55ff",
    "blue": "#8080ff",
    "gray": "#aaaaaa",
    "gold": "#ffd933",
    "red": "#ff5555",
}


def cell_x(col):
    return 7 + CELL * col


def cell_y(row):
    return 17 + CELL * row


class Menu:
    def __init__(self, menu_id, name, rows, title):
        self.data = {
            "formatVersion": 1,
            "id": menu_id,
            "name": name,
            "container": {"type": "chest", "rows": rows},
            "state": {},
            "layers": [{"id": "background", "texture": f"mcrs/panel_r{max(4, rows)}.png", "x": 0, "y": 0}],
            "texts": [{"id": "title", "x": 8, "y": 6, "color": TEXT["title"], "value": title}],
            "slots": [],
        }

    def layer(self, layer_id, texture, x, y, visible_when=None):
        layer = {"id": layer_id, "texture": f"mcrs/{texture}.png", "x": x, "y": y}
        if visible_when:
            layer["visibleWhen"] = visible_when
        self.data["layers"].append(layer)

    def text(self, text_id, x, y, value, color="light", align="left", visible_when=None):
        text = {"id": text_id, "x": x, "y": y, "value": value, "color": TEXT.get(color, color)}
        if align != "left":
            text["align"] = align
        if visible_when:
            text["visibleWhen"] = visible_when
        self.data["texts"].append(text)

    def slot(self, slot_id, col, row, width, height, label, on_click, enabled_when=None, kind="button"):
        area = {"col": col, "row": row}
        if width != 1:
            area["width"] = width
        if height != 1:
            area["height"] = height
        slot = {"id": slot_id, "kind": kind, "area": area, "item": {"invisible": True, "name": label}}
        if on_click:
            slot["onClick"] = on_click
        if enabled_when:
            slot["enabledWhen"] = enabled_when
        self.data["slots"].append(slot)

    def button(self, slot_id, col, row, cells, label, accent, on_click, enabled_when=None):
        """Bouton dessiné (couche) + libellé centré (texte) + zone de slots."""
        self.layer(f"{slot_id}_button", f"button_w{cells}_{accent}", cell_x(col), cell_y(row))
        self.text(f"{slot_id}_label", cell_x(col) + CELL * cells // 2 + 1, cell_y(row) + 5, label, accent, "center")
        self.slot(slot_id, col, row, cells, 1, f"<{MINI[accent]}>{label}", on_click, enabled_when)

    def save(self):
        os.makedirs(MENUS, exist_ok=True)
        path = os.path.join(MENUS, f"{self.data['id']}.menu.json")
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            json.dump(self.data, handle, ensure_ascii=False, indent=2)
            handle.write("\n")


MINI = {
    "green": "green",
    "aqua": "aqua",
    "yellow": "yellow",
    "pink": "light_purple",
    "blue": "blue",
    "gray": "gray",
    "gold": "gold",
    "red": "red",
}


def open_menu(menu_id):
    return [{"type": "open", "menu": menu_id}]


def command(line):
    """Commande du joueur, puis fermeture (comme les anciens formulaires)."""
    return [{"type": "command", "command": line, "as": "player"}, {"type": "close"}]


SHOWCASE = open_menu("showcase")


def hub():
    menu = Menu("hub", "Hub mc-rs", 5, "mc-rs HUB")
    menu.text("subtitle", 8, cell_y(0) + 5, f"Choisis une action{NBSP}:", "muted")
    buttons = [
        ("spawn", "Spawn", "green", command("tp {mcrs.spawn}")),
        ("creative", "Créatif", "aqua", command("gamemode creative")),
        ("survival", "Survie", "yellow", command("gamemode survival")),
        ("day", "Jour", "pink", command("time set day")),
        ("night", "Nuit", "blue", command("time set night")),
        ("biome", "Biome", "gray", command("biome")),
    ]
    for index, (slot_id, label, accent, actions) in enumerate(buttons):
        menu.button(slot_id, 0 if index % 2 == 0 else 5, 1 + index // 2, 4, label, accent, actions)
    menu.button("showcase", 0, 4, 9, "UI Showcase", "gold", open_menu("showcase"))
    return menu


def showcase():
    menu = Menu("showcase", "UI Showcase", 6, "UI Showcase")
    menu.text("subtitle", 8, cell_y(0) + 5, f"Sélectionne le layout{NBSP}:", "muted")
    demos = [
        ("grid", "Grid"),
        ("left_button", "Left buttons"),
        ("bottom_button", "Bottom"),
        ("image_grid", "Image grid"),
        ("square_image", "Square image"),
        ("motd", "MOTD"),
        ("store", "Store"),
        ("wrapped", "Wrapped"),
    ]
    for index, (menu_id, label) in enumerate(demos):
        menu.button(f"open_{menu_id}", 0 if index % 2 == 0 else 5, 1 + index // 2, 4, label, "yellow", open_menu(menu_id))
    menu.button("back", 0, 5, 4, "Retour", "red", open_menu("hub"))
    return menu


def grid():
    menu = Menu("grid", "Grid layout", 5, "Grid layout")
    menu.text("subtitle", 8, cell_y(0) + 5, "Grille de boutons", "muted")
    for index, letter in enumerate("ABCDEF"):
        menu.button(f"action_{letter.lower()}", 0 if index % 2 == 0 else 5, 1 + index // 2, 4, f"Action {letter}", "yellow", SHOWCASE)
    menu.button("back", 0, 4, 4, "Retour", "red", SHOWCASE)
    return menu


def left_button():
    menu = Menu("left_button", "Left buttons", 5, "Left buttons")
    for row, label in enumerate(["Partie classée", "Partie rapide", "Tutoriel", "Paramètres"]):
        menu.button(f"choice_{row}", 0, row, 5, label, "yellow", SHOWCASE)
    menu.button("back", 0, 4, 5, "Retour", "red", SHOWCASE)
    menu.layer("description", "description", cell_x(5) + 1, cell_y(0))
    for index, line in enumerate(["Liste à gauche,", "description", "à droite."]):
        menu.text(f"description_{index}", cell_x(5) + 5, cell_y(0) + 6 + 11 * index, line, "muted")
    return menu


def bottom_button():
    menu = Menu("bottom_button", "Bottom buttons", 6, "Bottom buttons")
    menu.layer("banner", "banner_bedwars", cell_x(0), cell_y(0))
    menu.text("banner_label", 88, cell_y(0) + 6, "Mode Bedwars", "light", "center")
    for index, label in enumerate(["Solo", "Duo", "Squad"]):
        menu.button(f"join_{label.lower()}", 3 * index, 3, 3, label, "yellow", [{"type": "close"}])
    menu.button("back", 0, 5, 4, "Retour", "red", SHOWCASE)
    return menu


def image_grid():
    menu = Menu("image_grid", "Image grid", 6, "Image grid")
    maps = [
        ("forest", "Forêt", "green", 0, 0),
        ("desert", "Désert", "yellow", 5, 0),
        ("mountain", "Montagne", "red", 0, 2),
        ("swamp", "Marais", "yellow", 5, 2),
    ]
    for kind, label, color, col, row in maps:
        menu.layer(f"map_{kind}", f"map_{kind}", cell_x(col), cell_y(row))
        menu.text(f"map_{kind}_label", cell_x(col) + 36, cell_y(row) + 26, label, color, "center")
        menu.slot(f"map_{kind}", col, row, 4, 2, f"<{MINI[color]}>{label}", SHOWCASE)
    menu.text("legend", 88, cell_y(4) + 5, "Facile, moyenne, difficile", "muted", "center")
    menu.button("back", 0, 5, 4, "Retour", "red", SHOWCASE)
    return menu


def square_image():
    menu = Menu("square_image", "Square image", 6, "Square image")
    menu.layer("image", "square_image", cell_x(2), cell_y(0))
    menu.slot("image", 2, 0, 5, 5, "Image", SHOWCASE)
    menu.text("caption", 88, cell_y(5) + 5, "Image carrée, texte en bas", "muted", "center")
    return menu


def motd():
    menu = Menu("motd", "MOTD", 6, "MOTD")
    menu.layer("banner", "banner_motd", cell_x(0), cell_y(0))
    menu.text("welcome", 88, cell_y(0) + 14, "Bienvenue sur mc-rs", "gold", "center")
    for index, line in enumerate(["Le serveur Bedrock le plus", "rapide, écrit en Rust.", "Appuie sur un bouton."]):
        menu.text(f"line_{index}", 88, cell_y(2) + 4 + 11 * index, line, "light" if index < 2 else "yellow", "center")
    menu.button("continue", 0, 5, 4, "Continuer", "green", SHOWCASE)
    menu.button("quit", 5, 5, 4, "Quitter", "red", SHOWCASE)
    return menu


def store():
    menu = Menu("store", "Store", 6, "Store")
    for index, label in enumerate(["Populaire", "Nouveautés", "Promotions"]):
        menu.layer(f"tab_{index}", "tab_on" if index == 0 else "tab_off", cell_x(3 * index), cell_y(0))
        menu.text(f"tab_{index}_label", cell_x(3 * index) + 27, cell_y(0) + 5, label, "gold" if index == 0 else "muted", "center")
        menu.slot(f"tab_{index}", 3 * index, 0, 3, 1, label, SHOWCASE)
    articles = [
        ("sword", "Épée légendaire", "1500"),
        ("pickaxe", "Pioche en diamant", "800"),
        ("skin", "Skin pirate", "2000"),
        ("cape", "Cape dragon", "3500"),
    ]
    for index, (kind, label, price) in enumerate(articles):
        row = 1 + index
        menu.layer(f"article_{kind}", f"store_{kind}", cell_x(0), cell_y(row))
        menu.text(f"article_{kind}_name", cell_x(0) + 24, cell_y(row) + 5, label, "light")
        menu.text(f"article_{kind}_price", cell_x(9) - 4, cell_y(row) + 5, f"{price} coins", "green", "right")
        menu.slot(f"article_{kind}", 0, row, 9, 1, f"{label}{NBSP}: {price} coins", SHOWCASE)
    return menu


def wrapped():
    menu = Menu("wrapped", "Wrapped", 6, "Wrapped")
    for index in range(3):
        menu.layer(f"image_{index + 1}", f"wrapped_{index + 1}", cell_x(3 * index), cell_y(0))
    menu.text("recap", 88, cell_y(3) + 4, "Ton récap 2026", "gold", "center")
    menu.text("link", 88, cell_y(4) + 2, "mcrs.io/recap/2026", "muted", "center")
    menu.button("continue", 0, 5, 4, "Continuer", "green", SHOWCASE)
    menu.button("close", 5, 5, 4, "Fermer", "red", SHOWCASE)
    return menu


def demo():
    """Menu propre à Menu Forge (/mf open demo) : états, couches conditionnelles, textes dynamiques."""
    menu = Menu("demo", "Démo Menu Forge", 6, "Démo Menu Forge")
    menu.data["state"] = {
        "tab": {"type": "enum", "values": ["infos", "settings"], "default": "infos"},
        "count": {"type": "int", "default": 0, "min": 0, "max": 5},
    }
    menu.text("greeting", 8, cell_y(0) + 5, "Bonjour {viewer.name}", "light")
    for col, tab_id, label in ((0, "infos", "Infos"), (5, "settings", "Réglages")):
        menu.layer(f"tab_{tab_id}_off", "button_w4_gray", cell_x(col), cell_y(1))
        menu.layer(f"tab_{tab_id}_on", "button_w4_active", cell_x(col), cell_y(1), {"state": "tab", "is": tab_id})
        menu.text(f"tab_{tab_id}_label", cell_x(col) + 37, cell_y(1) + 5, label, "gold", "center")
        menu.slot(
            f"tab_{tab_id}", col, 1, 4, 1, label,
            [{"type": "setState", "state": "tab", "value": tab_id}, {"type": "sound", "sound": "ui.button.click"}],
            {"not": {"state": "tab", "is": tab_id}},
        )
    menu.text("infos", 88, cell_y(2) + 5, "Onglet Infos : couches et textes", "muted", "center", {"state": "tab", "is": "infos"})
    menu.text("settings", 88, cell_y(2) + 5, "Onglet Réglages : compteur", "muted", "center", {"state": "tab", "is": "settings"})
    for value in range(6):
        menu.layer(f"count_{value}_button", "button_w1_yellow", cell_x(value), cell_y(3))
        menu.text(f"count_{value}_label", cell_x(value) + 10, cell_y(3) + 5, str(value), "yellow", "center")
        menu.slot(
            f"count_{value}", value, 3, 1, 1, str(value),
            [{"type": "setState", "state": "count", "value": value}],
            {"not": {"state": "count", "is": value}},
        )
    menu.layer("badge", "badge", cell_x(7) + 1, cell_y(3) + 1, {"state": "count", "in": [3, 4, 5]})
    menu.text("count", cell_x(9) - 4, cell_y(4) + 5, f"Compteur{NBSP}: {{state.count}}", "gold", "right")
    menu.button("chat", 0, 4, 4, "Chat", "aqua", [{"type": "command", "command": "me teste Menu Forge", "as": "player"}])
    menu.button("back", 0, 5, 4, "Retour", "red", [{"type": "back"}])
    menu.button("close", 5, 5, 4, "Fermer", "gray", [{"type": "close"}])
    return menu


def generate_menus():
    menus = [hub(), showcase(), grid(), left_button(), bottom_button(), image_grid(), square_image(), motd(), store(), wrapped(), demo()]
    for menu in menus:
        menu.save()
    print(f"{len(menus)} menus écrits dans {MENUS}")


if __name__ == "__main__":
    generate_textures()
    if "--menus" in sys.argv[1:]:
        generate_menus()
