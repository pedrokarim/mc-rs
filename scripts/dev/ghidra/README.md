# Scripts Ghidra : lecture de BDS

Depuis la fin de support de PocketMine-MP, la référence de comportement est le serveur officiel de Mojang
(Bedrock Dedicated Server, `.reference/bds/`). Ces scripts interrogent un projet Ghidra où BDS a été importé
et analysé **une seule fois** : l'analyse automatique d'un exécutable de plus de 200 Mo prend des heures, les
requêtes suivantes quelques minutes.

## Prérequis

- Ghidra 12.1.x (testé avec 12.1.4), JDK 21 ou plus récent ;
- BDS téléchargé depuis l'API officielle de Mojang
  (`https://net-secondary.web.minecraft-services.net/api/v1.0/download/links`), dans
  `.reference/bds/bedrock-server-<version>-win/`.

Le binaire n'a pas de symboles de fonctions, mais garde la RTTI : l'analyseur RTTI de Ghidra nomme les
classes et leurs vtables (`FenceBlock::vftable`…).

## 1. Import et analyse (une fois par version de BDS)

En PowerShell, hors du dépôt (le projet pèse plusieurs Go) :

```powershell
$env:JAVA_HOME = "C:\Program Files\Eclipse Adoptium\jdk-25.0.2.10-hotspot"
$env:GHIDRA_HEADLESS_MAXMEM = "16G"
C:\tools\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\tools\ghidra-projects bds-1.26.52.3 `
  -import <chemin>\bedrock_server.exe -max-cpu 12 -log C:\tools\ghidra-projects\bds-1.26.52.3-analyze.log
```

## 2. Requêtes

Sans réanalyse (`-noanalysis`) et sans modifier le projet (`-readOnly`) :

```powershell
C:\tools\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat C:\tools\ghidra-projects bds-1.26.52.3 `
  -process bedrock_server.exe -noanalysis -readOnly -scriptPath scripts\dev\ghidra `
  -postScript DecompileStringRefs.java <dossier_sortie> minecraft:connection_north inner_left
```

| Script | Arguments | Résultat |
|---|---|---|
| `DecompileStringRefs.java` | `<sortie> <chaîne>...` | fonctions qui référencent chaque chaîne, décompilées, et `index.txt` |
| `DecompileVtable.java` | `<sortie> <classe>...` | méthodes virtuelles de chaque classe (vtables RTTI), décompilées, et `vtable.txt` |

Le pseudo-code produit est une **lecture** du binaire, pas du code source : les noms de fonctions sont ceux
de Ghidra (`FUN_<adresse>`), et une conclusion se recoupe avec le comportement observé en jeu ou les
références (gophertunnel, dragonfly). Les sorties restent hors du dépôt.
