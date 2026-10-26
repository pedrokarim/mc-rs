// bds-capture observe un serveur Bedrock de référence (BDS) avec gophertunnel.
//
//	bds-capture capture [options]  : se connecte comme client, enregistre chaque paquet échangé ;
//	bds-capture decode [options]   : décode un enregistrement en JSON (une ligne par paquet) ;
//	bds-capture drops [options]    : lance BDS, fait calculer par BDS les drops de blocs (/loot ... mine) et
//	                                 relève les items apparus.
//
// Les enregistrements suivent le format de MCRS_DUMP_PACKETS=full de mc-rs (little-endian
// [u64 ms][u32 id][u32 longueur][charge utile]), ce qui permet de comparer BDS et mc-rs avec les mêmes outils.
// Voir README.md.
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage : bds-capture capture|decode|drops [options]  (-h pour l'aide de chaque commande)")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "capture":
		capture(os.Args[2:])
	case "decode":
		decode(os.Args[2:])
	case "drops":
		drops(os.Args[2:])
	default:
		fmt.Fprintf(os.Stderr, "commande inconnue : %s\n", os.Args[1])
		os.Exit(2)
	}
}
