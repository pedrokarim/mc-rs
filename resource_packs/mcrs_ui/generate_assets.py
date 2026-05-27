#!/usr/bin/env python3
"""Génère les assets de test pour le pack mc-rs UI.

Génère des PNG simples (carrés multicolores) pour valider le pipeline
ResourcePack et le rendu côté client. Aucun asset n'est copié — tout est
généré programmatiquement.

Sortie : ./pack_icon.png, ./textures/ui/mcrs/*.png

Usage : depuis le dossier `resource_packs/mcrs_ui/` :
    python3 generate_assets.py
"""

import os
from PIL import Image, ImageDraw, ImageFont

# ── Palette mc-rs (cohérente avec _global_variables.json) ────────────────
PANEL_BG       = (12, 12, 25, 245)        # bleu nuit semi-transparent
PANEL_BG_SOLID = (12, 12, 25, 255)
GOLD           = (255, 217, 51, 255)      # accent doré
BUTTON_DEFAULT = (30, 30, 46, 230)
BUTTON_HOVER   = (56, 56, 82, 250)
BUTTON_PRESSED = (77, 64, 13, 255)        # doré sombre
TEXT_LIGHT     = (240, 240, 240, 255)
TEXT_GOLD      = (255, 217, 51, 255)
ORANGE_HOT     = (255, 154, 30, 255)


def ensure_dir(p):
    os.makedirs(p, exist_ok=True)


def make_solid(path, size, color):
    img = Image.new("RGBA", size, color)
    img.save(path, "PNG")
    print(f"  {path}  {size[0]}×{size[1]}")


def make_button_9slice(path, color, border_color=None, border_px=1, size=(16, 16)):
    """Texture nine-slice scalable : bordure 1px + intérieur uni."""
    img = Image.new("RGBA", size, color)
    if border_color:
        d = ImageDraw.Draw(img)
        w, h = size
        for i in range(border_px):
            d.rectangle([i, i, w - 1 - i, h - 1 - i], outline=border_color)
    img.save(path, "PNG")
    print(f"  {path}  {size[0]}×{size[1]} (border={border_px}px)")


def make_pack_icon(path, size=128):
    """Icône carrée avec un fond bleu nuit + lettres « mc-rs » dorées
    centrées + un cadre doré. Tout pur PIL, aucune ressource externe."""
    img = Image.new("RGBA", (size, size), PANEL_BG_SOLID)
    d = ImageDraw.Draw(img)

    # Cadre doré (4 px d'épaisseur, intérieur transparent)
    for i in range(4):
        d.rectangle([i, i, size - 1 - i, size - 1 - i], outline=GOLD)

    # Petit liseré orangé décoratif intérieur (8 px du bord)
    for i in range(8, 10):
        d.rectangle([i, i, size - 1 - i, size - 1 - i], outline=ORANGE_HOT)

    # Texte « mc-rs » centré. On essaie une font système ; fallback bitmap.
    text = "mc-rs"
    font = None
    for candidate in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/Library/Fonts/Arial.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
    ]:
        try:
            font = ImageFont.truetype(candidate, size=int(size * 0.28))
            break
        except (OSError, IOError):
            continue
    if font is None:
        font = ImageFont.load_default()

    bbox = d.textbbox((0, 0), text, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    tx = (size - tw) / 2 - bbox[0]
    ty = (size - th) / 2 - bbox[1]
    # Ombre légère
    d.text((tx + 2, ty + 2), text, fill=(0, 0, 0, 200), font=font)
    d.text((tx, ty), text, fill=GOLD, font=font)

    img.save(path, "PNG")
    print(f"  {path}  {size}×{size} (icon)")


def main():
    base = os.path.dirname(os.path.abspath(__file__))
    os.chdir(base)
    tex_root = "textures/ui/mcrs"
    ensure_dir(tex_root)

    # ── pack_icon ─────────────────────────────────────────────────────
    make_pack_icon("pack_icon.png", size=128)

    # ── Panels (fonds pleins, étirés par UI JSON via "size":"100%") ──
    make_solid(f"{tex_root}/panel_bg.png", (8, 8), PANEL_BG)
    make_solid(f"{tex_root}/panel_bg_solid.png", (8, 8), PANEL_BG_SOLID)

    # Bandeau doré (étiré horizontalement par les UI JSON)
    make_solid(f"{tex_root}/strip_gold.png", (8, 4), GOLD)
    make_solid(f"{tex_root}/strip_orange.png", (8, 4), ORANGE_HOT)

    # ── Buttons nine-slice (bordure 1 px) ────────────────────────────
    make_button_9slice(
        f"{tex_root}/button_default.png",
        color=BUTTON_DEFAULT,
        border_color=(60, 60, 80, 255),
        size=(16, 16),
    )
    make_button_9slice(
        f"{tex_root}/button_hover.png",
        color=BUTTON_HOVER,
        border_color=GOLD,
        size=(16, 16),
    )
    make_button_9slice(
        f"{tex_root}/button_pressed.png",
        color=BUTTON_PRESSED,
        border_color=ORANGE_HOT,
        size=(16, 16),
    )

    print("Done.")


if __name__ == "__main__":
    main()
