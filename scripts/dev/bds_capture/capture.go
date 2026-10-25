package main

import (
	"flag"
	"log"
	"net"
	"sync"
	"time"

	"github.com/sandertv/gophertunnel/minecraft"
	"github.com/sandertv/gophertunnel/minecraft/protocol/packet"
)

// observed est un paquet vu par PacketFunc, avec son émetteur. La direction n'est connue qu'une fois
// l'adresse distante établie : les paquets de pré-spawn passent par PacketFunc avant que Dial ne rende la
// connexion.
type observed struct {
	record
	src string
}

func capture(args []string) {
	fs := flag.NewFlagSet("capture", flag.ExitOnError)
	network := fs.String("network", "nethernet", "réseau : nethernet (BDS 1.26.52+) ou raknet")
	addr := fs.String("addr", "http://127.0.0.1:19150", "adresse du serveur (URL de signalisation en NetherNet)")
	out := fs.String("out", "capture", "préfixe des fichiers produits : <out>_s2c.bin et <out>_c2s.bin")
	stay := fs.Duration("stay", 20*time.Second, "durée de présence en jeu après le spawn")
	tokenPath := fs.String("token", defaultTokenPath(), "jeton Microsoft enregistré (hors du dépôt)")
	fs.Parse(args)

	src, err := tokenSource(*tokenPath)
	if err != nil {
		log.Fatalf("authentification : %v", err)
	}

	start := time.Now()
	var mu sync.Mutex
	var seen []observed
	dialer := minecraft.Dialer{
		TokenSource: src,
		PacketFunc: func(h packet.Header, payload []byte, from, _ net.Addr) {
			mu.Lock()
			defer mu.Unlock()
			seen = append(seen, observed{
				record: record{
					ms:      uint64(time.Since(start).Milliseconds()),
					id:      h.PacketID,
					payload: append([]byte(nil), payload...),
				},
				src: from.String(),
			})
		},
	}
	conn, err := dialer.Dial(*network, *addr)
	if err != nil {
		log.Fatalf("connexion : %v", err)
	}
	remote := conn.RemoteAddr().String()
	log.Printf("connecté à %s (%s)", remote, *network)
	if err := conn.DoSpawn(); err != nil {
		log.Printf("spawn : %v", err)
	} else {
		log.Printf("en jeu, enregistrement pendant %s", *stay)
		deadline := time.Now().Add(*stay)
		for time.Now().Before(deadline) {
			conn.SetReadDeadline(deadline)
			if _, err := conn.ReadPacket(); err != nil {
				break
			}
		}
	}
	conn.Close()

	mu.Lock()
	defer mu.Unlock()
	var s2c, c2s []record
	for _, o := range seen {
		if o.src == remote {
			s2c = append(s2c, o.record)
		} else {
			c2s = append(c2s, o.record)
		}
	}
	if err := writeRecords(*out+"_s2c.bin", s2c); err != nil {
		log.Fatal(err)
	}
	if err := writeRecords(*out+"_c2s.bin", c2s); err != nil {
		log.Fatal(err)
	}
	log.Printf("paquets : %d serveur → client, %d client → serveur", len(s2c), len(c2s))
}
