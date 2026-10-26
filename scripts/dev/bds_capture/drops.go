package main

import (
	"bufio"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"maps"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"slices"
	"strings"
	"sync"
	"time"

	"github.com/sandertv/gophertunnel/minecraft"
	"github.com/sandertv/gophertunnel/minecraft/protocol/packet"
)

// Plan d'observation : chaque bloc est tiré `samples` fois avec chacun des outils.
type dropPlan struct {
	Tools  []dropTool `json:"tools"`
	Blocks []string   `json:"blocks"`
	// Pour chaque bloc, l'état vérifié par `execute if block` : les seules propriétés qui comptent, BDS
	// recalculant à la pose orientation et connexions. Nom seul quand la liste est absente.
	Checks  []string `json:"checks,omitempty"`
	Samples int      `json:"samples"`
}

// Outil tenu en main ; vide = main nue.
type dropTool struct {
	Tool     string         `json:"tool,omitempty"`
	Enchants map[string]int `json:"enchants,omitempty"`
}

// Un item apparu pour un tirage.
type droppedItem struct {
	Item  string `json:"item"`
	Meta  int32  `json:"meta,omitempty"`
	Count uint16 `json:"count"`
	// Runtime ID réseau du bloc pour un item bloc (hash : block-network-ids-are-hashes=true sur BDS).
	BlockHash int32 `json:"block_hash,omitempty"`
}

// Item apparu dans le monde (AddItemActor), avant attribution à une case.
type spawned struct {
	x, y, z float32
	item    droppedItem
}

// Résultat d'un tirage, une ligne JSON par tirage. `Placed` faux : `testforblock` n'a pas trouvé le bloc
// (pose refusée, bloc détruit par une mise à jour) ; le tirage ne vaut alors rien.
type dropSample struct {
	Block  string        `json:"block"`
	Tool   dropTool      `json:"tool"`
	Sample int           `json:"sample"`
	Placed bool          `json:"placed"`
	Items  []droppedItem `json:"items"`
}

const (
	// Blocs observés posés en grille à la hauteur blockY, chacun sur de la bedrock, à 2 blocs d'écart. En
	// altitude, loin du terrain : poser des blocs près du sol fait sauter herbes et fleurs, dont les items se
	// mêleraient au relevé.
	blockY       = 200
	blockSpacing = 2
	blockSide    = 15
	// Grille où chaque tirage fait apparaître ses items : une case par bloc, pour attribuer chaque item à
	// son tirage par sa position (apparition au centre de la case). 64 cases de côté : ±32 blocs. Au-delà
	// d'environ 3 chunks du joueur, BDS n'envoie pas les items au bot (mesuré : grille de ±60 blocs, coins
	// perdus).
	gridY       = 205
	gridSpacing = 1
	gridSide    = 64
	// Délai après le marqueur de fin de lot : BDS envoie les AddItemActor au tick suivant la commande.
	settleDelay = 1500 * time.Millisecond
	// Pack de comportement qui porte les fonctions du relevé. BDS n'exécute qu'une commande de console par
	// tick ; une fonction exécute toutes les siennes dans le même tick (limite : gamerule
	// functionCommandLimit, 10 000 par défaut). Le pack est écrit avant le démarrage de BDS, appliqué au monde
	// le temps du relevé, puis retiré pour ne pas fausser les autres captures (pile de packs).
	packDir     = "development_behavior_packs/mcrs_drops"
	packUUID    = "6f1c2b9e-3d4a-4e8b-9a71-5c0d2e8f4b13"
	moduleUUID  = "a2d7e4c1-8b3f-4f62-b05e-9e1a7c3d5f28"
	packVersion = "[1, 0, 0]"
)

// bds : processus BDS piloté par sa console (entrée standard), sortie recopiée dans un journal et
// transmise aux abonnés (réponses de `testforblock`).
type bds struct {
	cmd     *exec.Cmd
	stdin   io.WriteCloser
	started chan struct{}
	lines   chan string
}

func startBDS(exe, logPath string) (*bds, error) {
	cmd := exec.Command(exe)
	cmd.Dir = filepath.Dir(exe)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return nil, err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, err
	}
	logFile, err := os.Create(logPath)
	if err != nil {
		return nil, err
	}
	if err := cmd.Start(); err != nil {
		return nil, err
	}
	b := &bds{cmd: cmd, stdin: stdin, started: make(chan struct{}), lines: make(chan string, 4096)}
	go func() {
		defer logFile.Close()
		once := sync.Once{}
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := scanner.Text()
			fmt.Fprintln(logFile, line)
			if strings.Contains(line, "Server started.") {
				once.Do(func() { close(b.started) })
			}
			select {
			case b.lines <- line:
			default:
			}
		}
	}()
	return b, nil
}

func (b *bds) run(command string) {
	fmt.Fprintln(b.stdin, command)
}

func (b *bds) stop() {
	b.run("stop")
	done := make(chan error, 1)
	go func() { done <- b.cmd.Wait() }()
	select {
	case <-done:
	case <-time.After(30 * time.Second):
		b.cmd.Process.Kill()
	}
}

func blockCell(i int) (int, int) {
	half := blockSide / 2
	return (i%blockSide - half) * blockSpacing, (i/blockSide - half) * blockSpacing
}

func gridCell(i int) (float32, float32) {
	half := gridSide / 2
	x := (i%gridSide - half) * gridSpacing
	z := (i/gridSide - half) * gridSpacing
	return float32(x) + 0.5, float32(z) + 0.5
}

// Réponse de testforblock : « Successfully found the block at X,Y,Z. » ou un message d'échec qui cite
// aussi la position.
var foundRe = regexp.MustCompile(`Successfully found the block at (-?\d+),\s*(-?\d+),\s*(-?\d+)`)

func drops(args []string) {
	fs := flag.NewFlagSet("drops", flag.ExitOnError)
	exe := fs.String("bds", "", "chemin de bedrock_server.exe (BDS est lancé et arrêté par l'outil)")
	planPath := fs.String("plan", "", "fichier JSON : {tools, blocks, samples}")
	out := fs.String("out", "drops.jsonl", "résultats : une ligne JSON par tirage")
	network := fs.String("network", "nethernet", "réseau : nethernet (BDS 1.26.52+) ou raknet")
	addr := fs.String("addr", "http://127.0.0.1:19150", "adresse du serveur (URL de signalisation en NetherNet)")
	tokenPath := fs.String("token", defaultTokenPath(), "jeton Microsoft enregistré (hors du dépôt)")
	fs.Parse(args)

	planBytes, err := os.ReadFile(*planPath)
	if err != nil {
		log.Fatalf("plan : %v", err)
	}
	var plan dropPlan
	if err := json.Unmarshal(planBytes, &plan); err != nil {
		log.Fatalf("plan : %v", err)
	}
	if plan.Samples < 1 || len(plan.Blocks) == 0 || len(plan.Tools) == 0 {
		log.Fatal("plan : tools, blocks et samples (≥ 1) sont requis")
	}
	checks := map[string]string{}
	for i, block := range plan.Blocks {
		checks[block] = blockName(block)
		if plan.Checks != nil {
			if len(plan.Checks) != len(plan.Blocks) {
				log.Fatal("plan : checks doit avoir autant d'entrées que blocks")
			}
			checks[block] = plan.Checks[i]
		}
	}

	// Reprise : les couples bloc/outil déjà complets dans le fichier de sortie sont sautés, les nouveaux
	// tirages y sont ajoutés. Les noms de blocs que BDS a refusés (« Syntax error ») sont retenus à côté du
	// relevé et écartés des fonctions, qu'ils feraient rejeter entières.
	completed := previousSurvey(*out, plan.Samples)
	rejectedPath := *out + ".rejected.json"
	rejected := loadRejected(rejectedPath)

	// Lots : jusqu'à blockSide² blocs posés, et assez de cases pour samples tirages chacun.
	cells := gridSide * gridSide
	perBatch := max(1, min(blockSide*blockSide, cells/plan.Samples))
	samplesPerRound := min(plan.Samples, cells/perBatch)

	todos := make([][]string, len(plan.Tools))
	for t, tool := range plan.Tools {
		for _, block := range plan.Blocks {
			if !completed[pairKey(block, tool)] {
				todos[t] = append(todos[t], block)
			}
		}
	}
	// À partir d'ici, une erreur passe par fail puis return : les defer arrêtent BDS et retirent le pack du
	// monde (un log.Fatal le laisserait appliqué), puis le code de sortie vaut 1.
	failed := false
	defer func() {
		if failed {
			os.Exit(1)
		}
	}()
	fail := func(format string, v ...any) {
		log.Printf(format, v...)
		failed = true
	}
	restorePacks, err := installFunctionPack(*exe, todos, checks, rejected, perBatch, samplesPerRound, plan.Samples)
	if err != nil {
		fail("pack de fonctions : %v", err)
		return
	}
	defer restorePacks()

	server, err := startBDS(*exe, *out+".bds.log")
	if err != nil {
		fail("BDS : %v", err)
		return
	}
	defer server.stop()
	select {
	case <-server.started:
	case <-time.After(3 * time.Minute):
		fail("BDS n'a pas démarré en 3 minutes")
		return
	}
	log.Print("BDS démarré")

	src, err := tokenSource(*tokenPath)
	if err != nil {
		fail("authentification : %v", err)
		return
	}
	// La connexion NetherNet expire parfois (« context deadline exceeded ») : trois essais.
	var conn *minecraft.Conn
	for attempt := 1; attempt <= 3; attempt++ {
		if conn, err = (minecraft.Dialer{TokenSource: src}).DialTimeout(*network, *addr, time.Minute); err == nil {
			break
		}
		log.Printf("connexion, essai %d : %v", attempt, err)
	}
	if err != nil {
		fail("connexion impossible")
		return
	}
	defer conn.Close()
	if err := conn.DoSpawn(); err != nil {
		fail("spawn : %v", err)
		return
	}
	itemNames := map[int32]string{}
	for _, entry := range conn.GameData().Items {
		itemNames[int32(entry.RuntimeID)] = entry.Name
	}

	var mu sync.Mutex
	var pending []spawned
	// Blocs du lot trouvés en place au tick des tirages (`execute if block … run say mcrs-ok-<i>`).
	present := map[int]bool{}
	markers := make(chan string, 16)
	go func() {
		for {
			pk, err := conn.ReadPacket()
			if err != nil {
				return
			}
			switch p := pk.(type) {
			case *packet.AddItemActor:
				stack := p.Item.Stack
				mu.Lock()
				pending = append(pending, spawned{
					x: p.Position[0], y: p.Position[1], z: p.Position[2],
					item: droppedItem{
						Item:      itemNames[stack.NetworkID],
						Meta:      int32(stack.MetadataValue),
						Count:     stack.Count,
						BlockHash: stack.BlockRuntimeID,
					},
				})
				mu.Unlock()
			case *packet.Text:
				if strings.Contains(p.Message, "mcrs-mark-") {
					markers <- p.Message
				} else if m := presentRe.FindStringSubmatch(p.Message); m != nil {
					var i int
					fmt.Sscan(m[1], &i)
					mu.Lock()
					present[i] = true
					mu.Unlock()
				}
			}
		}
	}()

	markerID := 0
	barrier := func() {
		markerID++
		marker := fmt.Sprintf("mcrs-mark-%d", markerID)
		server.run("say " + marker)
		waitMarker(markers, marker)
	}

	// Joueur en survie au-dessus de la zone, zone chargée avant d'y poser quoi que ce soit. Les items
	// tombent ensuite librement : seule compte leur position d'apparition.
	for _, c := range []string{
		"gamemode survival @a",
		"gamerule doDaylightCycle false",
		"gamerule doMobSpawning false",
		"gamerule doTileDrops true",
		fmt.Sprintf("tp @a 0.5 %d 0.5", gridY),
	} {
		server.run(c)
	}
	time.Sleep(5 * time.Second)
	lo, _ := blockCell(0)
	hi, _ := blockCell(blockSide*blockSide - 1)
	server.run(fmt.Sprintf("fill %d %d %d %d %d %d bedrock", lo, blockY-1, lo, hi, blockY-1, hi))
	barrier()
	// Sans le pack, `function` échouerait et chaque tirage serait relevé vide : on vérifie qu'il est chargé.
	server.run("function " + checkFunction)
	select {
	case <-markers:
	case <-time.After(30 * time.Second):
		fail("pack de fonctions non chargé par BDS (voir le journal de BDS), relevé abandonné")
		return
	}
	// Étalonnage : BDS n'envoie au bot que les items des chunks qu'il lui a déjà transmis. Tant qu'une case
	// de la grille ne reçoit pas son item (terre cassée à la main, toujours 1 terre), le relevé attend.
	server.run("clear @a")
	cellsUsed := perBatch * samplesPerRound
	for attempt := 1; ; attempt++ {
		mu.Lock()
		pending = nil
		mu.Unlock()
		server.run("function " + calibrateFunction)
		barrier()
		time.Sleep(settleDelay)
		mu.Lock()
		got := pending
		pending = nil
		mu.Unlock()
		missing := 0
		for _, items := range assignCells(got, cellsUsed) {
			if len(items) != 1 || items[0].Item != "minecraft:dirt" || items[0].Count != 1 {
				missing++
			}
		}
		if missing == 0 {
			log.Printf("étalonnage : %d cases reçues", cellsUsed)
			break
		}
		if attempt == 10 {
			fail("étalonnage : %d cases sur %d sans leur item après 10 essais, relevé abandonné", missing, cellsUsed)
			return
		}
		log.Printf("étalonnage : %d cases sur %d sans leur item, nouvel essai", missing, cellsUsed)
		time.Sleep(3 * time.Second)
	}

	outFile, err := os.OpenFile(*out, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		fail("%v", err)
		return
	}
	defer outFile.Close()
	enc := json.NewEncoder(outFile)

	for t, tool := range plan.Tools {
		todo := todos[t]
		if len(todo) == 0 {
			log.Printf("outil %q %v : déjà relevé", tool.Tool, tool.Enchants)
			continue
		}
		server.run("clear @a")
		if tool.Tool != "" {
			server.run("give @a " + tool.Tool)
			for name, level := range tool.Enchants {
				server.run(fmt.Sprintf("enchant @a %s %d", name, level))
			}
		}
		if skipped := len(plan.Blocks) - len(todo); skipped > 0 {
			log.Printf("outil %q %v : %d blocs déjà relevés, sautés", tool.Tool, tool.Enchants, skipped)
		}
		for batch, start := 0, 0; start < len(todo); batch, start = batch+1, start+perBatch {
			blocks := todo[start:min(start+perBatch, len(todo))]
			for done := 0; done < plan.Samples; done += samplesPerRound {
				rounds := min(samplesPerRound, plan.Samples-done)
				mu.Lock()
				pending = nil
				clear(present)
				mu.Unlock()
				// Items précédents tués, pose, vérification de la pose et tirages du lot dans le même tick :
				// ce qui est relevé est bien ce que BDS a cassé, sans qu'une mise à jour de bloc (corail qui
				// meurt, poudre de béton qui durcit) s'intercale. Les cases des blocs absents d'un dernier lot
				// incomplet visent de l'air.
				draw := drawFunction(functionTool(todos, t), batch, rounds)
				drainLines(server)
				server.run("function " + draw)
				loaded := functionLoaded(server, draw)
				barrier()
				var placed []bool
				if !loaded {
					// BDS refuse de charger une fonction dont une seule ligne est invalide (bloc inconnu de
					// `setblock` ou de `execute if block`) : le lot passe par la console, une commande par tick,
					// et les noms refusés sont retenus pour les fonctions des relevés suivants.
					log.Printf("fonction %s non chargée par BDS : lot par la console", draw)
					server.run("kill @e[type=item]")
					placeBlocks(server, blocks, rejected, barrier)
					time.Sleep(300 * time.Millisecond)
					mu.Lock()
					pending = nil
					mu.Unlock()
					placed = testBlocks(server, blocks, checks, rejected, barrier)
					if err := saveRejected(rejectedPath, rejected); err != nil {
						log.Printf("noms refusés : %v", err)
					}
					server.run("function " + lootFunction(rounds))
					barrier()
				}
				time.Sleep(settleDelay)

				mu.Lock()
				got := pending
				pending = nil
				if placed == nil {
					placed = make([]bool, len(blocks))
					for b := range blocks {
						placed[b] = present[b]
					}
				}
				mu.Unlock()
				byCell := assignCells(got, len(blocks)*rounds)
				for b, block := range blocks {
					for r := range rounds {
						items := byCell[b*rounds+r]
						if items == nil {
							items = []droppedItem{}
						}
						enc.Encode(dropSample{Block: block, Tool: tool, Sample: done + r, Placed: placed[b], Items: items})
					}
				}
			}
		}
		log.Printf("outil %q %v : %d blocs, %d tirages chacun", tool.Tool, tool.Enchants, len(todo), plan.Samples)
	}
	server.run("kill @e[type=item]")
}

// assignCells attribue chaque item reçu à la case de la grille où il est apparu, parmi les n premières.
func assignCells(got []spawned, n int) [][]droppedItem {
	byCell := make([][]droppedItem, n)
	half := gridSide / 2
	for _, s := range got {
		// BDS fait apparaître les items de `loot spawn x y z` en (x - 0,5, z - 0,5) : mesuré sur 3 000 items,
		// sans exception. La case i est visée en gridCell(i), centre du bloc.
		gx := int(math.Round(float64(s.x)/gridSpacing)) + half
		gz := int(math.Round(float64(s.z)/gridSpacing)) + half
		i := gz*gridSide + gx
		if gx >= 0 && gx < gridSide && gz >= 0 && i < n {
			x, z := gridCell(i)
			dx, dz := s.x+0.5-x, s.z+0.5-z
			if dx*dx+dz*dz < 0.25*gridSpacing*gridSpacing && math.Abs(float64(s.y-gridY)) < 1.0 {
				byCell[i] = append(byCell[i], s.item)
				continue
			}
		}
		log.Printf("item hors grille ignoré : %+v à (%.2f, %.2f, %.2f)", s.item, s.x, s.y, s.z)
	}
	return byCell
}

// placeBlocks pose le lot par la console : couche vidée, puis chaque bloc sur la bedrock (une commande par
// tick).
//
// Les noms que `setblock` refuse sont ajoutés à rejected : certains (`end_gateway`) passent `testforblock` et
// `execute if block` mais pas `setblock`.
func placeBlocks(server *bds, blocks []string, rejected map[string]bool, barrier func()) {
	drainLines(server)
	lines := placeLines(blocks, nil)
	for _, line := range lines {
		server.run(line)
	}
	barrier()
	// Une réponse par commande (fill, puis chaque setblock), lue en différé comme pour testforblock.
	answers := 0
	timeout := time.After(10 * time.Second)
	for answers < len(lines) {
		select {
		case line := <-server.lines:
			switch {
			case rejectedRe.MatchString(line):
				rejected[normalizeBlock(rejectedRe.FindStringSubmatch(line)[1])] = true
				answers++
			case strings.Contains(line, "Block placed"), strings.Contains(line, "Unable to place block"),
				strings.Contains(line, "blocks filled"), strings.Contains(line, "The block couldn't be placed"):
				answers++
			}
		case <-timeout:
			log.Printf("setblock : %d réponses sur %d en 10 s", answers, len(lines))
			answers = len(lines)
		}
	}
}

// placeLines : vidage de la couche puis `setblock` de chaque bloc du lot, sauf ceux que skip écarte.
func placeLines(blocks []string, skip func(string) bool) []string {
	lo, _ := blockCell(0)
	hi, _ := blockCell(blockSide*blockSide - 1)
	lines := []string{fmt.Sprintf("fill %d %d %d %d %d %d air", lo, blockY, lo, hi, blockY, hi)}
	for i, block := range blocks {
		if skip != nil && skip(block) {
			continue
		}
		bx, bz := blockCell(i)
		lines = append(lines, fmt.Sprintf("setblock %d %d %d %s", bx, blockY, bz, block))
	}
	return lines
}

// testBlocks vérifie la présence de chaque bloc du lot avec testforblock (une commande par tick) et ajoute à
// rejected les noms que la commande refuse.
func testBlocks(server *bds, blocks []string, checks map[string]string, rejected map[string]bool, barrier func()) []bool {
	drainLines(server)
	for i, block := range blocks {
		bx, bz := blockCell(i)
		server.run(fmt.Sprintf("testforblock %d %d %d %s", bx, blockY, bz, checks[block]))
	}
	barrier()
	// La sortie de BDS arrive en différé (tampon du tube) : on attend une réponse par commande (trouvé, autre
	// bloc ou erreur de syntaxe), pas seulement le marqueur reçu par le réseau.
	found := map[[2]int]bool{}
	answers := 0
	timeout := time.After(10 * time.Second)
	for answers < len(blocks) {
		select {
		case line := <-server.lines:
			switch {
			case foundRe.MatchString(line):
				m := foundRe.FindStringSubmatch(line)
				var x, z int
				fmt.Sscan(m[1], &x)
				fmt.Sscan(m[3], &z)
				found[[2]int{x, z}] = true
				answers++
			case strings.Contains(line, "The block at "):
				answers++
			case rejectedRe.MatchString(line):
				rejected[normalizeBlock(rejectedRe.FindStringSubmatch(line)[1])] = true
				answers++
			}
		case <-timeout:
			log.Printf("testforblock : %d réponses sur %d en 10 s", answers, len(blocks))
			answers = len(blocks)
		}
	}
	placed := make([]bool, len(blocks))
	for i := range blocks {
		bx, bz := blockCell(i)
		placed[i] = found[[2]int{bx, bz}]
	}
	return placed
}

// blockName retire les états d'un bloc du plan (`nom[états]`).
func blockName(block string) string {
	return strings.TrimSpace(strings.SplitN(block, "[", 2)[0])
}

func drainLines(server *bds) {
	for len(server.lines) > 0 {
		<-server.lines
	}
}

// functionLoaded attend la réponse de BDS à `function <name>` : exécutée, ou « Function … not found » quand
// BDS n'a pas pu la charger.
func functionLoaded(server *bds, name string) bool {
	timeout := time.After(10 * time.Second)
	for {
		select {
		case line := <-server.lines:
			if strings.Contains(line, "Function "+name+" not found") {
				return false
			}
			if strings.Contains(line, "Successfully executed") {
				return true
			}
		case <-timeout:
			log.Printf("pas de réponse de BDS à la fonction %s en 10 s", name)
			return false
		}
	}
}

// Réponse de BDS à un nom de bloc qu'il ne connaît pas : « Syntax error: Unexpected "minecraft:…" ».
var rejectedRe = regexp.MustCompile(`Syntax error: Unexpected "([^"]+)"`)

func normalizeBlock(name string) string {
	return "minecraft:" + strings.TrimPrefix(blockName(name), "minecraft:")
}

func loadRejected(path string) map[string]bool {
	rejected := map[string]bool{}
	data, err := os.ReadFile(path)
	if err != nil {
		return rejected
	}
	var names []string
	if err := json.Unmarshal(data, &names); err != nil {
		log.Fatalf("%s : %v", path, err)
	}
	for _, name := range names {
		rejected[name] = true
	}
	return rejected
}

func saveRejected(path string, rejected map[string]bool) error {
	names := slices.Sorted(maps.Keys(rejected))
	data, err := json.MarshalIndent(names, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, append(data, '\n'), 0o644)
}

const (
	checkFunction     = "mcrs/check"
	calibrateFunction = "mcrs/calibrate"
)

var presentRe = regexp.MustCompile(`mcrs-ok-(\d+)`)

// functionTool : premier outil dont la liste de blocs à relever est celle de l'outil t. Les fonctions de
// tirage ne dépendent pas de l'outil tenu ; elles ne sont écrites qu'une fois par liste (un relevé complet
// en écrivait sinon une copie par outil, 46 Mo que BDS mettait 30 s à charger).
func functionTool(todos [][]string, t int) int {
	for first := range t {
		if slices.Equal(todos[first], todos[t]) {
			return first
		}
	}
	return t
}

func drawFunction(tool, batch, rounds int) string {
	return fmt.Sprintf("mcrs/draw_t%d_b%d_r%d", tool, batch, rounds)
}

func lootFunction(rounds int) string {
	return fmt.Sprintf("mcrs/loot_r%d", rounds)
}

// installFunctionPack écrit le pack des fonctions du relevé dans le dossier de BDS et l'applique au monde
// (`worlds/<level-name>/world_behavior_packs.json`). La fonction renvoyée rétablit le fichier du monde tel
// qu'il était ; si l'outil est tué avant, le pack reste appliqué jusqu'au relevé suivant.
func installFunctionPack(exe string, todos [][]string, checks map[string]string, rejected map[string]bool, perBatch, samplesPerRound, samples int) (func(), error) {
	root := filepath.Dir(exe)
	dir := filepath.Join(root, filepath.FromSlash(packDir))
	if err := os.RemoveAll(dir); err != nil {
		return nil, err
	}
	functions := filepath.Join(dir, "functions", "mcrs")
	if err := os.MkdirAll(functions, 0o755); err != nil {
		return nil, err
	}
	manifest := fmt.Sprintf(`{
  "format_version": 2,
  "header": {
    "name": "mc-rs drops",
    "description": "Fonctions du relevé des drops (bds-capture drops)",
    "uuid": "%s",
    "version": %s,
    "min_engine_version": [1, 26, 50]
  },
  "modules": [{"type": "data", "uuid": "%s", "version": %s}]
}
`, packUUID, packVersion, moduleUUID, packVersion)
	if err := os.WriteFile(filepath.Join(dir, "manifest.json"), []byte(manifest), 0o644); err != nil {
		return nil, err
	}
	write := func(name string, lines []string) error {
		path := filepath.Join(dir, "functions", filepath.FromSlash(name)+".mcfunction")
		return os.WriteFile(path, []byte(strings.Join(lines, "\n")+"\n"), 0o644)
	}

	if err := write(checkFunction, []string{"say mcrs-mark-pack"}); err != nil {
		return nil, err
	}
	dirt := make([]string, perBatch)
	for i := range dirt {
		dirt[i] = "minecraft:dirt"
	}
	roundCounts := []int{samplesPerRound}
	if rest := samples % samplesPerRound; rest != 0 {
		roundCounts = append(roundCounts, rest)
	}
	lootLines := map[int][]string{}
	for _, rounds := range roundCounts {
		for b := range perBatch {
			bx, bz := blockCell(b)
			for r := range rounds {
				x, z := gridCell(b*rounds + r)
				lootLines[rounds] = append(lootLines[rounds], fmt.Sprintf(
					"execute as @a at @s run loot spawn %.1f %d %.1f mine %d %d %d mainhand", x, gridY, z, bx, blockY, bz))
			}
		}
		if err := write(lootFunction(rounds), lootLines[rounds]); err != nil {
			return nil, err
		}
	}
	calibrate := append([]string{"kill @e[type=item]"}, placeLines(dirt, nil)...)
	if err := write(calibrateFunction, append(calibrate, lootLines[samplesPerRound]...)); err != nil {
		return nil, err
	}
	for t, todo := range todos {
		if functionTool(todos, t) != t {
			continue
		}
		for batch, start := 0, 0; start < len(todo); batch, start = batch+1, start+perBatch {
			blocks := todo[start:min(start+perBatch, len(todo))]
			// Un nom refusé par BDS ferait rejeter toute la fonction.
			unplaceable := func(block string) bool { return rejected[normalizeBlock(block)] }
			lines := append([]string{"kill @e[type=item]"}, placeLines(blocks, unplaceable)...)
			for i, block := range blocks {
				if !unplaceable(block) {
					bx, bz := blockCell(i)
					lines = append(lines, fmt.Sprintf("execute if block %d %d %d %s run say mcrs-ok-%d", bx, blockY, bz, checks[block], i))
				}
			}
			for _, rounds := range roundCounts {
				if err := write(drawFunction(t, batch, rounds), append(slices.Clone(lines), lootLines[rounds]...)); err != nil {
					return nil, err
				}
			}
		}
	}

	level, err := levelName(filepath.Join(root, "server.properties"))
	if err != nil {
		return nil, err
	}
	worldPacks := filepath.Join(root, "worlds", level, "world_behavior_packs.json")
	previous, readErr := os.ReadFile(worldPacks)
	applied := fmt.Sprintf(`[{"pack_id": "%s", "version": %s}]`+"\n", packUUID, packVersion)
	if readErr == nil && string(previous) == applied {
		// Reste d'un relevé interrompu : le monde n'avait pas de fichier avant lui.
		readErr = os.ErrNotExist
	}
	if err := os.WriteFile(worldPacks, []byte(applied), 0o644); err != nil {
		return nil, err
	}
	return func() {
		if readErr == nil {
			os.WriteFile(worldPacks, previous, 0o644)
		} else {
			os.Remove(worldPacks)
		}
	}, nil
}

func levelName(properties string) (string, error) {
	data, err := os.ReadFile(properties)
	if err != nil {
		return "", err
	}
	for _, line := range strings.Split(string(data), "\n") {
		if name, ok := strings.CutPrefix(strings.TrimSpace(line), "level-name="); ok {
			return name, nil
		}
	}
	return "", fmt.Errorf("level-name absent de %s", properties)
}

// pairKey identifie un couple bloc/outil (enchantements compris).
func pairKey(block string, tool dropTool) string {
	key, _ := json.Marshal(tool)
	return block + "|" + string(key)
}

// previousSurvey lit un relevé existant et renvoie les couples bloc/outil qui ont tous leurs tirages.
func previousSurvey(path string, samples int) map[string]bool {
	counts := map[string]int{}
	f, err := os.Open(path)
	if err != nil {
		return map[string]bool{}
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 1<<20), 1<<20)
	for scanner.Scan() {
		var row dropSample
		if json.Unmarshal(scanner.Bytes(), &row) == nil {
			counts[pairKey(row.Block, row.Tool)]++
		}
	}
	done := map[string]bool{}
	for key, n := range counts {
		if n >= samples {
			done[key] = true
		}
	}
	return done
}

func waitMarker(markers <-chan string, want string) {
	timeout := time.After(30 * time.Second)
	for {
		select {
		case m := <-markers:
			if strings.Contains(m, want) {
				return
			}
		case <-timeout:
			log.Printf("marqueur %s non reçu en 30 s", want)
			return
		}
	}
}
