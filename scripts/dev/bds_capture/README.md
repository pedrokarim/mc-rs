# bds-capture

Observe un serveur Bedrock de référence (BDS) avec gophertunnel, et produit des enregistrements comparables
à ceux de mc-rs (`MCRS_DUMP_PACKETS=full`). Méthode et constats : [`dev-docs/34-BDS-RETRO-INGENIERIE.md`](../../../dev-docs/34-BDS-RETRO-INGENIERIE.md).

## Construire

Go 1.25 ou plus récent :

```powershell
cd scripts\dev\bds_capture
$env:GOARCH = "amd64"; go build -o bds-capture.exe .
```

## Utiliser

```text
bds-capture capture -network nethernet -addr http://127.0.0.1:19150 -out <préfixe> -stay 20s
bds-capture capture -network raknet -addr 127.0.0.1:19132 -out <préfixe> -stay 20s
bds-capture decode -in <préfixe>_s2c.bin -direction s2c -out <préfixe>_s2c.jsonl
```

- `capture` se connecte comme un client et écrit `<préfixe>_s2c.bin` (serveur → client) et `<préfixe>_c2s.bin`
  (client → serveur), au format `[u64 ms][u32 id][u32 longueur][charge utile]` little-endian.
- `decode` décode chaque paquet avec gophertunnel et écrit une ligne JSON par paquet ; `unread` signale des
  octets restants (format divergent), `error` un échec de décodage.
- La première connexion demande un code sur https://www.microsoft.com/link (compte Microsoft du jeu) : BDS
  exige une identité Xbox Live en NetherNet, même avec `online-mode=false`. Le jeton est enregistré dans
  `%APPDATA%\mc-rs\bds-capture-token.json`, hors du dépôt.
