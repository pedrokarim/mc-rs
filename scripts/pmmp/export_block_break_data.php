<?php

/**
 * Exporte depuis PocketMine-MP les données de cassage de chaque état de la palette de mc-rs.
 *
 * Pour chaque état de `canonical_block_states.nbt` (indice = runtime ID, `blockNetworkIdsAreHashes=false`),
 * le bloc est désérialisé par PMMP lui-même, puis on relève :
 *   - `BlockBreakInfo` : dureté, résistance aux explosions, type d'outil (bits `BlockToolType`), niveau de
 *     récolte ;
 *   - `Block::getDrops()` pour chaque famille d'outil (main nue, puis outils en netherite, cisailles, Toucher
 *     de soie, Fortune III) ; une liste fixe si le résultat ne varie pas, sinon `"random"` (règle portée
 *     classe par classe dans mc-rs) ;
 *   - l'XP (`getXpDropForTool` avec un outil compatible) : constante, ou bornes observées si aléatoire.
 *
 * Les outils (efficacité, niveau de récolte, durabilité) sont exportés de la même façon depuis les items de
 * PMMP.
 *
 * Usage (PHP fourni avec PMMP, qui a l'extension `encoding`) :
 *   .reference/PocketMine-MP/bin/php/php.exe -d memory_limit=4G scripts/pmmp/export_block_break_data.php
 *
 * Sortie : crates/mc-rs-server/data/block_break.json. Méthode : dev-docs/36-CASSAGE-DE-BLOCS.md.
 */

declare(strict_types=1);

$root = dirname(__DIR__, 2);
$pmmp = $root . '/.reference/PocketMine-MP';
require $pmmp . '/vendor/autoload.php';

use pocketmine\block\BaseBanner;
use pocketmine\block\Bed;
use pocketmine\block\Block;
use pocketmine\block\BlockToolType;
use pocketmine\block\FlowerPot;
use pocketmine\block\ItemFrame;
use pocketmine\block\Lectern;
use pocketmine\block\ShulkerBox;
use pocketmine\data\bedrock\block\BlockStateData;
use pocketmine\item\Durable;
use pocketmine\item\enchantment\EnchantmentInstance;
use pocketmine\item\enchantment\VanillaEnchantments;
use pocketmine\item\Item;
use pocketmine\item\VanillaItems;
use pocketmine\nbt\NbtDataException;
use pocketmine\network\mcpe\protocol\serializer\NetworkNbtSerializer;
use pocketmine\VersionInfo;
use pocketmine\world\format\io\GlobalBlockStateHandlers;
use pocketmine\world\format\io\GlobalItemDataHandlers;

/** Tirages par scénario pour les classes dont le code de drop appelle l'aléatoire. */
const RANDOM_SAMPLES = 400;

/**
 * Propriétés ajoutées en 1.26.50 (worldupgrader v1.0.22, `0351_1.26.40_to_1.26.50.27.json`, `addedProperties`),
 * inconnues de PMMP (1.26.30). Ce sont des états de voisinage (forme d'escalier, connexions de clôture et de
 * vitre) : ils ne changent ni la dureté ni les drops. Retirées avant la désérialisation, puis remises à la
 * valeur par défaut du schéma sur les items blocs que PMMP renvoie comme drops.
 */
const IGNORED_PROPERTIES = [
	'minecraft:corner' => 'none',
	'minecraft:connection_north' => 0,
	'minecraft:connection_east' => 0,
	'minecraft:connection_south' => 0,
	'minecraft:connection_west' => 0,
];

$palettePath = $root . '/crates/mc-rs-server/data/canonical_block_states.nbt';
$outputPath = $root . '/crates/mc-rs-server/data/block_break.json';

/**
 * Clé canonique d'un état : nom + propriétés triées, valeurs typées.
 */
function stateKey(BlockStateData $data) : string{
	$states = [];
	foreach($data->getStates() as $k => $tag){
		$states[$k] = $tag->getValue();
	}
	return stateKeyFromValues($data->getName(), $states);
}

/** @param array<string, mixed> $states */
function stateKeyFromValues(string $name, array $states) : string{
	ksort($states);
	return $name . json_encode($states);
}

// ── Palette de mc-rs ──
$palette = [];
$keyToRuntimeId = [];
/** Propriétés de IGNORED_PROPERTIES présentes dans la palette, par nom de bloc. */
$ignoredByName = [];
try{
	foreach((new NetworkNbtSerializer())->readMultiple(file_get_contents($palettePath)) as $i => $root_){
		$data = BlockStateData::fromNbt($root_->mustGetCompoundTag());
		$palette[$i] = $data;
		$keyToRuntimeId[stateKey($data)] = $i;
		foreach($data->getStates() as $k => $_){
			if(array_key_exists($k, IGNORED_PROPERTIES)){
				$ignoredByName[$data->getName()][$k] = IGNORED_PROPERTIES[$k];
			}
		}
	}
}catch(NbtDataException $e){
	fwrite(STDERR, "palette illisible : " . $e->getMessage() . "\n");
	exit(1);
}
fwrite(STDERR, count($palette) . " états dans la palette de mc-rs\n");

$itemSerializer = GlobalItemDataHandlers::getSerializer();
$unresolvedBlockItems = [];

/**
 * Item → description stable : nom Bedrock, méta, nombre, état de bloc (runtime ID mc-rs) pour un item bloc.
 *
 * @return array<string, mixed>
 */
function describeItem(Item $item) : array{
	global $itemSerializer, $keyToRuntimeId, $unresolvedBlockItems, $ignoredByName;
	$saved = $itemSerializer->serializeType($item);
	$out = ['item' => $saved->getName()];
	if($saved->getMeta() !== 0){
		$out['meta'] = $saved->getMeta();
	}
	if($item->getCount() !== 1){
		$out['count'] = $item->getCount();
	}
	$block = $saved->getBlock();
	if($block !== null){
		$values = [];
		foreach($block->getStates() as $k => $tag){
			$values[$k] = $tag->getValue();
		}
		$values += $ignoredByName[$block->getName()] ?? [];
		$key = stateKeyFromValues($block->getName(), $values);
		if(isset($keyToRuntimeId[$key])){
			$out['block'] = $keyToRuntimeId[$key];
		}else{
			$unresolvedBlockItems[$key] = true;
			$out['block_state'] = $key;
		}
	}
	if($saved->getTag() !== null && $saved->getTag()->count() > 0){
		$out['nbt'] = true;
	}
	return $out;
}

/** @param Item[] $drops */
function describeDrops(array $drops) : array{
	$out = [];
	foreach($drops as $drop){
		if(!$drop->isNull()){
			$out[] = describeItem($drop);
		}
	}
	return $out;
}

/**
 * Vrai si le code de la classe (parents et traits compris) fait appel à l'aléatoire.
 */
function usesRandom(string $class) : bool{
	static $cache = [];
	if(isset($cache[$class])){
		return $cache[$class];
	}
	$sources = [];
	for($r = new ReflectionClass($class); $r !== false; $r = $r->getParentClass()){
		$sources[] = $r->getFileName();
		foreach($r->getTraits() as $trait){
			$sources[] = $trait->getFileName();
		}
	}
	foreach($sources as $file){
		$code = file_get_contents($file);
		if(preg_match('/mt_rand|FortuneDropHelper|\bRandom\b|random_int|array_rand/', $code) === 1){
			return $cache[$class] = true;
		}
	}
	return $cache[$class] = false;
}

/** Outils de référence : le meilleur de chaque famille, ce qui satisfait tout niveau de récolte. */
function toolFor(string $family) : Item{
	return match($family){
		'hand' => VanillaItems::AIR(),
		'pickaxe' => VanillaItems::NETHERITE_PICKAXE(),
		'axe' => VanillaItems::NETHERITE_AXE(),
		'shovel' => VanillaItems::NETHERITE_SHOVEL(),
		'hoe' => VanillaItems::NETHERITE_HOE(),
		'sword' => VanillaItems::NETHERITE_SWORD(),
		'shears' => VanillaItems::SHEARS(),
	};
}

const FAMILY_BITS = [
	'pickaxe' => BlockToolType::PICKAXE,
	'axe' => BlockToolType::AXE,
	'shovel' => BlockToolType::SHOVEL,
	'hoe' => BlockToolType::HOE,
	'sword' => BlockToolType::SWORD,
	'shears' => BlockToolType::SHEARS,
];

/** Outil enchanté pour Toucher de soie / Fortune : de la famille requise, sinon une pioche. */
function enchantedTool(Block $block, string $enchantment) : Item{
	$family = 'pickaxe';
	foreach(FAMILY_BITS as $name => $bit){
		if(($block->getBreakInfo()->getToolType() & $bit) !== 0){
			$family = $name;
			break;
		}
	}
	$tool = toolFor($family);
	$tool->addEnchantment(match($enchantment){
		'silk' => new EnchantmentInstance(VanillaEnchantments::SILK_TOUCH(), 1),
		'fortune' => new EnchantmentInstance(VanillaEnchantments::FORTUNE(), 3),
	});
	return $tool;
}

/**
 * Drops pour un outil : liste fixe, `"random"` si deux tirages diffèrent, `"world"` si PMMP a besoin du monde
 * (contenu d'une tuile : boîte de Shulker, bannière…).
 *
 * @return array<int, array<string, mixed>>|string
 */
function sampleDrops(Block $block, Item $tool, int $samples) : array|string{
	try{
		$first = describeDrops($block->getDrops(clone $tool));
	}catch(\pocketmine\utils\AssumptionFailedError){
		return 'world';
	}
	for($i = 1; $i < $samples; ++$i){
		if(describeDrops($block->getDrops(clone $tool)) !== $first){
			return 'random';
		}
	}
	return $first;
}

/**
 * @return int|array{min: int, max: int}
 */
function sampleXp(Block $block, Item $tool, int $samples) : int|array{
	$min = PHP_INT_MAX;
	$max = PHP_INT_MIN;
	for($i = 0; $i < $samples; ++$i){
		$xp = $block->getXpDropForTool(clone $tool);
		$min = min($min, $xp);
		$max = max($max, $xp);
	}
	return $min === $max ? $min : ['min' => $min, 'max' => $max];
}

/**
 * Drops de `getDropsForIncompatibleTool` (ou de son équivalent aléatoire).
 *
 * @return array<int, array<string, mixed>>|string
 */
function sampleIncompatibleDrops(Block $block, Item $tool, int $samples) : array|string{
	$first = describeDrops($block->getDropsForIncompatibleTool(clone $tool));
	for($i = 1; $i < $samples; ++$i){
		if(describeDrops($block->getDropsForIncompatibleTool(clone $tool)) !== $first){
			return 'random';
		}
	}
	return $first;
}

/**
 * Blocs dont PMMP lit les drops dans la tuile (`readStateFromWorld`) : couleur et motifs de bannière, couleur
 * de lit, plante du pot, item du cadre, livre du pupitre, contenu de la boîte de Shulker.
 */
function dropsDependOnTile(Block $block) : bool{
	return $block instanceof BaseBanner || $block instanceof Bed || $block instanceof FlowerPot
		|| $block instanceof ItemFrame || $block instanceof Lectern || $block instanceof ShulkerBox;
}

/** Outil Toucher de soie d'une famille que le bloc n'accepte pas (chemin « outil incompatible »). */
function incompatibleSilkTool(Block $block) : Item{
	$tool = ($block->getBreakInfo()->getToolType() & BlockToolType::PICKAXE) === 0 ? toolFor('pickaxe') : toolFor('shovel');
	$tool->addEnchantment(new EnchantmentInstance(VanillaEnchantments::SILK_TOUCH(), 1));
	return $tool;
}

// ── Données de BDS (dump d'Allay) ──
$allayRoot = $root . '/.reference/Allay';
$bdsStates = json_decode(file_get_contents($allayRoot . '/data/resources/block_states.json'), true, flags: JSON_THROW_ON_ERROR);
$bdsTypes = json_decode(file_get_contents($allayRoot . '/data/resources/block_types.json'), true, flags: JSON_THROW_ON_ERROR);
if(count($bdsStates) !== count($palette)){
	fwrite(STDERR, "dump de BDS : " . count($bdsStates) . " états, palette : " . count($palette) . "\n");
	exit(1);
}

/** Tags de type d'outil de BDS → bits `BlockToolType`. */
const BDS_TOOL_TAGS = [
	'minecraft:is_pickaxe_item_destructible' => BlockToolType::PICKAXE,
	'minecraft:is_axe_item_destructible' => BlockToolType::AXE,
	'minecraft:is_shovel_item_destructible' => BlockToolType::SHOVEL,
	'minecraft:is_hoe_item_destructible' => BlockToolType::HOE,
	'minecraft:is_sword_item_destructible' => BlockToolType::SWORD,
	'minecraft:is_shears_item_destructible' => BlockToolType::SHEARS,
];

/** Tags de rang de BDS → niveau de récolte de PMMP (`ToolTier::getHarvestLevel`). */
const BDS_TIER_TAGS = [
	'minecraft:stone_tier_destructible' => 3,
	'minecraft:iron_tier_destructible' => 4,
	'minecraft:diamond_tier_destructible' => 5,
];

/**
 * Blocs que PMMP déclare « outil requis » alors que BDS non, et pour lesquels BDS fait foi : le bloc se
 * récupère à la main (œuf de dragon, blocs de chimie).
 */
const BDS_HARVEST_OVERRIDES = [
	'minecraft:dragon_egg',
	'minecraft:lab_table',
	'minecraft:compound_creator',
	'minecraft:element_constructor',
	'minecraft:material_reducer',
];

/**
 * Blocs que PMMP déclare « outil requis » alors que BDS non, mais où le niveau de PMMP modélise les drops :
 * sans l'outil, PMMP passe par `getDropsForIncompatibleTool` (graines, bâtons, éclats). Le niveau de PMMP est
 * gardé pour les drops ; la vitesse suit `requires_correct_tool` de BDS.
 */
const PMMP_DROP_MODEL_CLASSES = [
	'AmethystCluster',
	'DeadBush',
	'DoubleTallGrass',
	'HangingRoots',
	'NetherSprouts',
	'SmallDripleaf',
	'TallGrass',
];

/**
 * Données de BDS d'un état : dureté, résistance (échelle de PMMP, ×5), bits d'outil, niveau de récolte.
 *
 * @return array{hardness: float, blast_resistance: float, tool_type: int, requires_correct_tool: bool, harvest_level: int}
 */
function bdsBreakInfo(int $runtimeId) : array{
	global $bdsStates, $bdsTypes;
	$state = $bdsStates[$runtimeId];
	$tags = $bdsTypes[$state['name']]['tags'] ?? [];
	$toolType = 0;
	$tier = 0;
	foreach($tags as $tag){
		$toolType |= BDS_TOOL_TAGS[$tag] ?? 0;
		$tier = max($tier, BDS_TIER_TAGS[$tag] ?? 0);
	}
	$requires = $state['requiresCorrectToolForDrops'];
	return [
		// Le dump est en float32 (1.799999 pour 1.8) : arrondi à 4 décimales.
		'hardness' => round($state['hardness'], 4),
		'blast_resistance' => round($state['explosionResistance'] * 5, 4),
		'tool_type' => $toolType,
		'requires_correct_tool' => $requires,
		'harvest_level' => $requires ? max($tier, 1) : 0,
	];
}

// ── Profils de cassage ──
$deserializer = GlobalBlockStateHandlers::getDeserializer();
$profiles = [];
$profileIndex = [];
$states = [];
$unmatched = [];
$harvestDecisions = [];
$differences = ['hardness' => [], 'blast_resistance' => [], 'tool_type' => []];

foreach($palette as $runtimeId => $data){
	if($bdsStates[$runtimeId]['name'] !== $data->getName()){
		fwrite(STDERR, "dump de BDS désaligné au runtime ID $runtimeId\n");
		exit(1);
	}
	$bds = bdsBreakInfo($runtimeId);

	$known = array_diff_key($data->getStates(), IGNORED_PROPERTIES);
	try{
		$block = $deserializer->deserializeBlock(BlockStateData::current($data->getName(), $known));
	}catch(\Throwable $e){
		// Bloc inconnu de PMMP : données de BDS seules, drops à porter à part.
		$unmatched[$data->getName()][] = $e->getMessage();
		$profile = ['source' => 'bds'] + $bds;
		$key = json_encode($profile);
		if(!isset($profileIndex[$key])){
			$profileIndex[$key] = count($profiles);
			$profiles[] = $profile;
		}
		$states[$runtimeId] = $profileIndex[$key];
		continue;
	}

	$info = $block->getBreakInfo();
	$class = (new ReflectionClass($block))->getShortName();
	$samples = usesRandom($block::class) ? RANDOM_SAMPLES : 2;

	// Niveau de récolte : celui de BDS quand BDS exige un outil, sinon celui de PMMP s'il modélise les drops.
	$harvestLevel = $bds['harvest_level'];
	$pmmpHarvest = $info->getToolHarvestLevel();
	$pmmpRequires = $info->getToolType() !== BlockToolType::NONE && $pmmpHarvest > 0;
	if($pmmpRequires && !$bds['requires_correct_tool']){
		if(in_array($class, PMMP_DROP_MODEL_CLASSES, true)){
			$harvestLevel = $pmmpHarvest;
		}elseif(!in_array($data->getName(), BDS_HARVEST_OVERRIDES, true)){
			fwrite(STDERR, "écart non tranché : {$data->getName()} ($class) exige un outil chez PMMP, pas chez BDS\n");
			exit(1);
		}
	}
	if($harvestLevel !== $pmmpHarvest){
		$harvestDecisions[$data->getName()] = ['pmmp' => $pmmpHarvest, 'retained' => $harvestLevel];
	}

	// Écarts entre PMMP et BDS, gardés dans le fichier pour mémoire (valeurs PMMP, puis BDS).
	$name = $data->getName();
	if(abs($info->getHardness() - $bds['hardness']) > 1e-4){
		$differences['hardness'][$name] = [$info->getHardness(), $bds['hardness']];
	}
	if(abs($info->getBlastResistance() - $bds['blast_resistance']) > 1e-3){
		$differences['blast_resistance'][$name] = [$info->getBlastResistance(), $bds['blast_resistance']];
	}
	if($info->getToolType() !== $bds['tool_type']){
		$differences['tool_type'][$name] = [$info->getToolType(), $bds['tool_type']];
	}

	$profile = [
		'source' => 'pmmp',
		'pmmp' => $block->getName(),
		'class' => $class,
		'hardness' => $bds['hardness'],
		'blast_resistance' => $bds['blast_resistance'],
		// Bits de BDS ; les cisailles viennent de PMMP (BDS les traite par tags de matière, cf. Allay
		// `ItemBaseComponentImpl::isCorrectToolFor` : laine, feuilles, plantes).
		'tool_type' => $bds['tool_type'] | ($info->getToolType() & BlockToolType::SHEARS),
		'requires_correct_tool' => $bds['requires_correct_tool'],
		'harvest_level' => $harvestLevel,
	];
	if($bds['hardness'] >= 0 && $info->isBreakable()){
		$handIncompatible = $pmmpRequires && $harvestLevel === 0;
		$drops = [
			// Main nue rendue compatible par BDS : drops du chemin compatible.
			'hand' => $handIncompatible ? describeDrops($block->getDropsForCompatibleTool(VanillaItems::AIR())) : sampleDrops($block, toolFor('hand'), $samples),
		];
		foreach(FAMILY_BITS as $family => $_){
			$drops[$family] = sampleDrops($block, toolFor($family), $samples);
		}
		$drops['silk_touch'] = sampleDrops($block, enchantedTool($block, 'silk'), $samples);
		$drops['incompatible'] = sampleIncompatibleDrops($block, VanillaItems::AIR(), $samples);
		$drops['incompatible_silk_touch'] = sampleIncompatibleDrops($block, incompatibleSilkTool($block), $samples);
		if(dropsDependOnTile($block)){
			// Exporté hors monde, le bloc a les valeurs par défaut de sa tuile : drops sans valeur.
			$drops = array_map(fn($d) => is_array($d) && count($d) > 0 ? 'world' : $d, $drops);
		}
		$profile['drops'] = $drops;
		$profile['affected_by_silk_touch'] = $block->isAffectedBySilkTouch();

		$compatible = $info->getToolType() === BlockToolType::NONE ? toolFor('hand') : enchantedTool($block, 'fortune');
		$compatible->removeEnchantments();
		$xp = sampleXp($block, $compatible, max($samples, 2));
		if($xp !== 0){
			$profile['xp'] = $xp;
		}
	}

	$key = json_encode($profile);
	if(!isset($profileIndex[$key])){
		$profileIndex[$key] = count($profiles);
		$profiles[] = $profile;
	}
	$states[$runtimeId] = $profileIndex[$key];
}

// ── Outils ──
$tools = [];
foreach(VanillaItems::getAll() as $item){
	$type = $item->getBlockToolType();
	$durability = $item instanceof Durable ? $item->getMaxDurability() : null;
	if($type === BlockToolType::NONE && $item->getMiningEfficiency(true) === 1.0 && $item->getMiningEfficiency(false) === 1.0){
		continue;
	}
	try{
		$name = $itemSerializer->serializeType($item)->getName();
	}catch(\Throwable){
		continue;
	}
	$tools[$name] = [
		'tool_type' => $type,
		'harvest_level' => $item->getBlockToolHarvestLevel(),
		'efficiency_correct' => $item->getMiningEfficiency(true),
		'efficiency_incorrect' => $item->getMiningEfficiency(false),
		'max_durability' => $durability,
	];
}
ksort($tools);

ksort($unmatched);
ksort($harvestDecisions);
foreach($differences as &$list){
	ksort($list);
}
unset($list);
$allayCommit = trim((string) shell_exec('git -C ' . escapeshellarg($allayRoot) . ' rev-parse --short HEAD'));
$output = [
	'source' => [
		'pmmp_version' => VersionInfo::BASE_VERSION,
		'bds_dump' => "Allay $allayCommit, data/resources/block_states.json + block_types.json",
		'palette' => 'crates/mc-rs-server/data/canonical_block_states.nbt',
		'palette_states' => count($palette),
		'generated_by' => 'scripts/pmmp/export_block_break_data.php',
	],
	'harvest_level_decisions' => $harvestDecisions,
	'pmmp_bds_differences' => $differences,
	'unknown_to_pmmp' => array_map(fn(array $errors) => ['states' => count($errors), 'error' => $errors[0]], $unmatched),
	'unresolved_block_items' => array_keys($unresolvedBlockItems),
	'tools' => $tools,
	'profiles' => $profiles,
	'states' => $states,
];
file_put_contents($outputPath, json_encode($output, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_PRESERVE_ZERO_FRACTION) . "\n");

$unmatchedStates = array_sum(array_map(fn(array $e) => count($e), $unmatched));
fwrite(STDERR, sprintf(
	"%d profils, %d états couverts, %d états inconnus de PMMP, données de BDS (%d blocs), %d items blocs non résolus, %d outils\n",
	count($profiles),
	count($palette) - $unmatchedStates,
	$unmatchedStates,
	count($unmatched),
	count($unresolvedBlockItems),
	count($tools)
));
fwrite(STDERR, "écrit $outputPath\n");
