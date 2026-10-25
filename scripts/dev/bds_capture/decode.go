package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"reflect"
	"strconv"
	"strings"

	"github.com/sandertv/gophertunnel/minecraft/protocol"
	"github.com/sandertv/gophertunnel/minecraft/protocol/packet"
)

// decodedLine est une ligne JSON produite par decode.
type decodedLine struct {
	Index  int    `json:"index"`
	Ms     uint64 `json:"ms"`
	ID     uint32 `json:"id"`
	Name   string `json:"name"`
	Size   int    `json:"size"`
	Error  string `json:"error,omitempty"`
	Unread int    `json:"unread,omitempty"`
	Packet any    `json:"packet,omitempty"`
}

func decode(args []string) {
	fs := flag.NewFlagSet("decode", flag.ExitOnError)
	in := fs.String("in", "", "enregistrement à décoder (.bin)")
	direction := fs.String("direction", "s2c", "s2c (paquets du serveur) ou c2s (paquets du client)")
	ids := fs.String("ids", "", "identifiants à décoder, séparés par des virgules (vide : tous)")
	out := fs.String("out", "", "fichier JSON Lines produit (vide : sortie standard)")
	typed := fs.Bool("typed", false, "NBT typé et champs non exportés (contenu des optionnels), voir typed.go")
	fs.Parse(args)
	if *in == "" {
		log.Fatal("-in est requis")
	}

	records, err := readRecords(*in)
	if err != nil {
		log.Printf("lecture : %v (%d enregistrement(s) lus)", err, len(records))
	}
	wanted := map[uint32]bool{}
	for _, s := range strings.Split(*ids, ",") {
		if s = strings.TrimSpace(s); s != "" {
			n, err := strconv.ParseUint(s, 0, 32)
			if err != nil {
				log.Fatalf("identifiant invalide : %s", s)
			}
			wanted[uint32(n)] = true
		}
	}
	pool := packet.NewServerPool()
	if *direction == "c2s" {
		pool = packet.NewClientPool()
	}
	shieldID := findShieldID(records, pool)

	w := os.Stdout
	if *out != "" {
		f, err := os.Create(*out)
		if err != nil {
			log.Fatal(err)
		}
		defer f.Close()
		w = f
	}
	enc := json.NewEncoder(w)
	decoded, failed := 0, 0
	for i, r := range records {
		if len(wanted) > 0 && !wanted[r.id] {
			continue
		}
		line := decodeOne(i, r, pool, shieldID)
		if *typed && line.Packet != nil {
			line.Packet = typedTree(reflect.ValueOf(line.Packet))
		}
		if line.Error != "" {
			failed++
		} else {
			decoded++
		}
		if err := enc.Encode(line); err != nil {
			line.Packet = nil
			line.Error = "encodage JSON : " + err.Error()
			enc.Encode(line)
		}
	}
	log.Printf("%d paquet(s) décodé(s), %d en échec (shield id %d)", decoded, failed, shieldID)
}

// decodeOne décode une charge utile avec le décodeur de gophertunnel. Les octets non lus à la fin sont
// signalés : un décodage « réussi » qui laisse des octets indique un format divergent.
func decodeOne(index int, r record, pool packet.Pool, shieldID int32) (line decodedLine) {
	line = decodedLine{Index: index, Ms: r.ms, ID: r.id, Size: len(r.payload)}
	ctor, ok := pool[r.id]
	if !ok {
		line.Name = "?"
		line.Error = "identifiant inconnu de gophertunnel"
		return line
	}
	pk := ctor()
	line.Name = fmt.Sprintf("%T", pk)[len("*packet."):]
	buf := bytes.NewBuffer(r.payload)
	defer func() {
		if p := recover(); p != nil {
			line.Error = fmt.Sprint("panique : ", p)
			line.Packet = nil
		}
	}()
	pk.Marshal(protocol.NewReader(buf, shieldID, false))
	line.Unread = buf.Len()
	line.Packet = pk
	return line
}

// findShieldID lit le runtime ID de minecraft:shield dans l'ItemRegistry, nécessaire au décodage des items.
func findShieldID(records []record, pool packet.Pool) int32 {
	ctor, ok := pool[packet.IDItemRegistry]
	if !ok {
		return 0
	}
	for _, r := range records {
		if r.id != packet.IDItemRegistry {
			continue
		}
		pk := ctor().(*packet.ItemRegistry)
		func() {
			defer func() { recover() }()
			pk.Marshal(protocol.NewReader(bytes.NewBuffer(r.payload), 0, false))
		}()
		for _, item := range pk.Items {
			if item.Name == "minecraft:shield" {
				return int32(item.RuntimeID)
			}
		}
	}
	return 0
}
