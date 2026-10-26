"""Plan du relevé des drops de BDS par état de bloc (`bds-capture drops`).

Les états de la palette sont regroupés par nom et par propriétés susceptibles de changer les drops ; les
propriétés d'orientation, de connexion aux voisins ou d'affichage (liste `IGNORED`) sont ignorées. Chaque
groupe est relevé sur un état représentant (le premier runtime ID du groupe). La correspondance état →
représentant est écrite à côté du plan, pour `build_bds_drops.py`.

Usage :
    python scripts/dev/plan_bds_drops.py --out .reference/bds/drops/plan-states-v2.json [--samples 16]
        [--only <relevé.jsonl>]   # ne garder que les blocs dont un état est aléatoire dans ce relevé

Méthode : dev-docs/36-CASSAGE-DE-BLOCS.md.
"""

import argparse
import collections
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bds_palette import command_block, load_palette  # noqa: E402

# Propriétés sans effet sur les drops : orientation, voisinage, alimentation en redstone, phases
# d'animation, contenu porté par une tuile. Une propriété absente de cette liste est relevée valeur par valeur.
IGNORED = {
    "facing_direction", "minecraft:facing_direction", "minecraft:cardinal_direction", "minecraft:block_face",
    "direction", "weirdo_direction", "ground_sign_direction", "torch_facing_direction", "coral_direction",
    "coral_fan_direction", "lever_direction", "rail_direction", "rotation", "pillar_axis", "portal_axis",
    "orientation", "attachment", "vine_direction_bits", "minecraft:vertical_half", "upside_down_bit",
    "minecraft:corner", "minecraft:connection_north", "minecraft:connection_east", "minecraft:connection_south",
    "minecraft:connection_west", "wall_connection_type_north", "wall_connection_type_east",
    "wall_connection_type_south", "wall_connection_type_west", "wall_post_bit", "pale_moss_carpet_side_north",
    "pale_moss_carpet_side_east", "pale_moss_carpet_side_south", "pale_moss_carpet_side_west", "attached_bit",
    "open_bit", "in_wall_bit", "door_hinge_bit", "powered_bit", "powered_shelf_type", "redstone_signal",
    "button_pressed_bit", "triggered_bit", "toggle_bit", "rail_data_bit", "conditional_bit", "output_lit_bit",
    "output_subtract_bit", "repeater_delay", "disarmed_bit", "suspended_bit", "occupied_bit", "crafting",
    "update_bit", "persistent_bit", "item_frame_map_bit", "item_frame_photo_bit", "big_dripleaf_tilt",
    "stability", "stability_check", "sculk_sensor_phase", "dripstone_thickness", "age_bit", "kelp_age",
    "weeping_vines_age", "twisting_vines_age", "growing_plant_age", "honey_level", "lit", "extinguished",
    "brushed_progress", "infiniburn_bit", "books_stored", "brewing_stand_slot_a_bit", "brewing_stand_slot_b_bit",
    "brewing_stand_slot_c_bit", "end_portal_eye_bit", "cauldron_liquid", "fill_level", "liquid_depth",
    "vault_state", "trial_spawner_state", "ominous",
}

# Blocs techniques ou impossibles à casser par un joueur (air, liquides, feu, portails, pistons en
# mouvement) : pas de drops à relever, et `setblock` refuse plusieurs d'entre eux, ce qui ferait rejeter
# toute la fonction de leur lot.
SKIPPED = {
    "minecraft:air", "minecraft:bubble_column", "minecraft:client_request_placeholder_block",
    "minecraft:end_gateway", "minecraft:end_portal", "minecraft:fire", "minecraft:soul_fire",
    "minecraft:flowing_lava", "minecraft:flowing_water", "minecraft:lava", "minecraft:water",
    "minecraft:moving_block", "minecraft:piston_arm_collision", "minecraft:sticky_piston_arm_collision",
    "minecraft:portal", "minecraft:unknown",
}

TOOLS = [
    {},
    {"tool": "wooden_pickaxe"}, {"tool": "stone_pickaxe"}, {"tool": "iron_pickaxe"},
    {"tool": "diamond_pickaxe"}, {"tool": "netherite_pickaxe"},
    {"tool": "wooden_axe"}, {"tool": "wooden_shovel"}, {"tool": "wooden_hoe"}, {"tool": "wooden_sword"},
    {"tool": "mace"}, {"tool": "shears"},
    # Enchantements sur une pioche en netherite, compatible avec tout bloc qui exige une pioche.
    {"tool": "netherite_pickaxe", "enchants": {"silk_touch": 1}},
    {"tool": "netherite_pickaxe", "enchants": {"fortune": 1}},
    {"tool": "netherite_pickaxe", "enchants": {"fortune": 2}},
    {"tool": "netherite_pickaxe", "enchants": {"fortune": 3}},
]


def groups(palette: list[dict]) -> tuple[list[int], list[int]]:
    """Représentants (runtime IDs) et, pour chaque état, l'indice de son représentant."""
    first: dict[tuple, int] = {}
    representative = []
    for rid, state in enumerate(palette):
        key = (state["name"], tuple(sorted((k, v) for k, v in state["states"].items() if k not in IGNORED)))
        representative.append(first.setdefault(key, rid))
    return sorted(set(representative)), representative


def random_blocks(survey: Path) -> set[str]:
    outcomes = collections.defaultdict(set)
    with survey.open(encoding="utf-8") as f:
        for line in f:
            row = json.loads(line)
            if row["placed"]:
                key = (row["block"], json.dumps(row["tool"], sort_keys=True))
                outcomes[key].add(json.dumps(sorted((i["item"], i["count"]) for i in row["items"])))
    return {block.split(" ")[0] for (block, _), outs in outcomes.items() if len(outs) > 1}


def main() -> None:
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=16)
    parser.add_argument("--only", type=Path, help="relevé : ne garder que les blocs aléatoires")
    args = parser.parse_args()

    palette = load_palette()
    reps, representative = groups(palette)
    reps = [rid for rid in reps if palette[rid]["name"] not in SKIPPED]
    if args.only:
        keep = random_blocks(args.only)
        reps = [rid for rid in reps if palette[rid]["name"] in keep]
    # Vérification de la pose sur les seules propriétés relevées : BDS recalcule les autres (connexions,
    # coins d'escaliers) en posant le bloc.
    checks = [
        command_block(palette[rid]["name"], {k: v for k, v in palette[rid]["states"].items() if k not in IGNORED})
        for rid in reps
    ]
    plan = {
        "samples": args.samples,
        "tools": TOOLS,
        "blocks": [palette[rid]["command"] for rid in reps],
        "checks": checks,
    }
    args.out.write_text(json.dumps(plan, indent=1) + "\n", encoding="utf-8")
    groups_path = args.out.with_suffix(".groups.json")
    groups_path.write_text(json.dumps({"representative": representative}) + "\n", encoding="utf-8")
    names = {palette[rid]["name"] for rid in reps}
    print(f"{len(reps)} états représentants ({len(names)} blocs) sur {len(palette)}, {len(TOOLS)} outils, "
          f"{args.samples} tirages : {len(reps) * len(TOOLS) * args.samples} tirages")
    print(f"écrit {args.out} et {groups_path}")


main()
