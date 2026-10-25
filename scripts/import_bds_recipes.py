"""Importe les recettes de BDS depuis une capture décodée en mode typé (`bds-capture decode -typed`).

Lit le premier CraftingData et l'ItemRegistry d'une capture serveur → client et écrit
`crates/mc-rs-server/data/recipes/bds.json` : toutes les recettes que BDS envoie au client (façonnées,
sans forme, à données utilisateur, multiples, forge, potions, contenants), avec leurs identifiants texte,
UUID, priorités, blocs et identifiants réseau. Les items sont désignés par leur nom (les identifiants réseau
de l'ItemRegistry de la capture sont traduits), jamais par runtime ID de bloc.

Le mode typé est requis : il garde les types NBT des sorties (feux d'artifice, bannières) et le contenu
des champs optionnels (condition de déblocage).

Usage :
    python scripts/import_bds_recipes.py <capture>/s2c.typed.jsonl --version 1.26.52.3
    python scripts/import_bds_recipes.py <capture>/s2c.typed.jsonl --version 1.26.52.3 --check

Méthode et capture de référence : dev-docs/34-BDS-RETRO-INGENIERIE.md.
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "crates" / "mc-rs-server" / "data" / "recipes" / "bds.json"

def first_packet(path: Path, name: str) -> dict:
    with path.open(encoding="utf-8") as f:
        for line in f:
            d = json.loads(line)
            if d["name"] == name:
                if d.get("error") or d.get("unread"):
                    sys.exit(f"{name} mal décodé dans la capture : {d.get('error') or d.get('unread')} octet(s)")
                return d["packet"]
    sys.exit(f"{name} absent de la capture {path}")


class Converter:
    def __init__(self, item_registry: dict):
        self.names = {e["RuntimeID"]: e["Name"] for e in item_registry["Items"]}

    def item_name(self, network_id: int) -> str:
        if network_id not in self.names:
            sys.exit(f"identifiant réseau d'item inconnu de l'ItemRegistry : {network_id}")
        return self.names[network_id]

    @staticmethod
    def ingredient(raw: dict) -> dict | None:
        """`ItemDescriptorCount` → `{"item", "meta", "count"}`, `{"tag", "count"}` ou None (case vide)."""
        desc = raw["Descriptor"]
        if not desc:
            if raw["Count"] != 0:
                sys.exit(f"descripteur vide avec une quantité : {raw}")
            return None
        if set(desc) == {"Name", "MetadataValue"}:
            return {"item": desc["Name"], "meta": desc["MetadataValue"], "count": raw["Count"]}
        if set(desc) == {"Tag"}:
            return {"tag": desc["Tag"], "count": raw["Count"]}
        sys.exit(f"descripteur d'ingrédient non pris en charge : {desc}")

    def output(self, raw: dict) -> dict:
        """`ItemStack` de sortie → `{"item", "meta", "count", "nbt"?}`, NBT typé (`{"compound": …}`)."""
        for key in ("CanBePlacedOn", "CanBreak"):
            if raw.get(key):
                sys.exit(f"sortie avec {key}, non pris en charge : {raw}")
        if raw.get("BlockingTick"):
            sys.exit(f"sortie avec BlockingTick, non pris en charge : {raw}")
        item = {
            "item": self.item_name(raw["ItemType"]["NetworkID"]),
            "meta": raw["ItemType"]["MetadataValue"],
            "count": raw["Count"],
            # BDS ne renseigne le bloc que pour les items-blocs (pas pour un lit, une porte, un panneau…).
            # Son runtime ID est un hash : seule sa présence est reprise.
            "block": raw["BlockRuntimeID"] != 0,
        }
        nbt = raw.get("NBTData") or {}
        if "compound" not in nbt:
            sys.exit(f"NBT de sortie non typé (décoder avec -typed) : {raw}")
        if nbt["compound"]:
            item["nbt"] = nbt
        return item

    def unlock(self, raw) -> dict | None:
        """Condition de déblocage (optionnel gophertunnel) : None si absente, sinon contexte et ingrédients
        dont la possession débloque la recette (contexte 0)."""
        if raw is None or not raw.get("set"):
            return None
        val = raw["val"]
        return {
            "context": val["Context"],
            "ingredients": [self.ingredient(i) for i in val.get("Ingredients") or []],
        }

    def common(self, r: dict) -> dict:
        return {
            "id": r["RecipeID"],
            "net_id": r["RecipeNetworkID"],
            "uuid": r["UUID"],
            "block": r["Block"],
            "priority": r["Priority"],
            "unlock": self.unlock(r.get("UnlockRequirement")),
        }

    def shaped(self, r: dict) -> dict:
        width, height = r["Width"], r["Height"]
        if len(r["Input"]) != width * height:
            sys.exit(f"recette façonnée incohérente : {r['RecipeID']}")
        return {
            **self.common(r),
            "width": width,
            "height": height,
            "assume_symmetry": r["AssumeSymmetry"],
            "input": [self.ingredient(i) for i in r["Input"]],
            "output": [self.output(o) for o in r["Output"]],
        }

    def shapeless(self, r: dict) -> dict:
        # UserDataShapelessRecipe intègre un ShapelessRecipe : la forme typée ne l'aplatit pas.
        r = r.get("ShapelessRecipe", r)
        return {
            **self.common(r),
            "input": [self.ingredient(i) for i in r["Input"]],
            "output": [self.output(o) for o in r["Output"]],
        }

    def smithing(self, r: dict, with_result: bool) -> dict:
        out = {
            "id": r["RecipeID"],
            "net_id": r["RecipeNetworkID"],
            "block": r["Block"],
            "template": self.ingredient(r["Template"]),
            "base": self.ingredient(r["Base"]),
            "addition": self.ingredient(r["Addition"]),
        }
        if with_result:
            out["output"] = self.output(r["Result"])
        return out

    def potion(self, r: dict) -> dict:
        return {
            "input": {"item": self.item_name(r["InputPotionID"]), "meta": r["InputPotionMetadata"]},
            "reagent": {"item": self.item_name(r["ReagentItemID"]), "meta": r["ReagentItemMetadata"]},
            "output": {"item": self.item_name(r["OutputPotionID"]), "meta": r["OutputPotionMetadata"]},
        }

    def container(self, r: dict) -> dict:
        return {
            "input": self.item_name(r["InputItemID"]),
            "reagent": self.item_name(r["ReagentItemID"]),
            "output": self.item_name(r["OutputItemID"]),
        }


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("capture", type=Path, help="s2c.jsonl produit par bds-capture decode")
    parser.add_argument("--version", required=True, help="version de BDS capturée, ex. 1.26.52.3")
    parser.add_argument("--check", action="store_true", help="compare au fichier en place sans l'écrire")
    args = parser.parse_args()

    crafting = first_packet(args.capture, "CraftingData")
    conv = Converter(first_packet(args.capture, "ItemRegistry"))
    for unsupported in ("ShapelessChemistryRecipes", "ShapedChemistryRecipes", "MaterialReducers"):
        if crafting.get(unsupported):
            sys.exit(f"{unsupported} non vide : format à ajouter au convertisseur")

    data = {
        "source": {
            "bds_version": args.version,
            "capture": args.capture.as_posix().split(".reference/", 1)[-1],
            "generated_by": "scripts/import_bds_recipes.py",
        },
        "shaped": [conv.shaped(r) for r in crafting["ShapedRecipes"]],
        "shapeless": [conv.shapeless(r) for r in crafting["ShapelessRecipes"]],
        "user_data_shapeless": [conv.shapeless(r) for r in crafting["UserDataShapelessRecipes"]],
        "multi": [{"uuid": r["UUID"], "net_id": r["RecipeNetworkID"]} for r in crafting["MultiRecipes"]],
        "smithing_transform": [conv.smithing(r, True) for r in crafting["SmithingTransformRecipes"]],
        "smithing_trim": [conv.smithing(r, False) for r in crafting["SmithingTrimRecipes"]],
        "potion_mixes": [conv.potion(r) for r in crafting["PotionRecipes"]],
        "container_mixes": [conv.container(r) for r in crafting["PotionContainerChangeRecipes"]],
    }

    net_ids = [r["net_id"] for k in ("shaped", "shapeless", "user_data_shapeless", "multi",
                                     "smithing_transform", "smithing_trim") for r in data[k]]
    if len(set(net_ids)) != len(net_ids) or 0 in net_ids:
        sys.exit("identifiants réseau de recette en double ou nuls")

    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    counts = {k: len(v) for k, v in data.items() if isinstance(v, list)}
    blocks = Counter(r["block"] for k in ("shaped", "shapeless", "user_data_shapeless") for r in data[k])
    print(f"BDS {args.version} : {counts}")
    print(f"blocs : {dict(blocks)}")
    if args.check:
        same = OUTPUT.exists() and OUTPUT.read_text(encoding="utf-8") == text
        print("identique au fichier en place" if same else "DIFFÉRENT du fichier en place")
        sys.exit(0 if same else 1)
    OUTPUT.write_text(text, encoding="utf-8", newline="\n")
    print(f"écrit {OUTPUT}")


main()
