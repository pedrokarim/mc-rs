#!/usr/bin/env bash
# Suit une auto-analyse Ghidra en cours (lancée par REA ou à la main) : une ligne à chaque changement
# d'analyseur, avec temps CPU et mémoire, une ligne de rappel toutes les 15 minutes, et une ligne finale
# quand la JVM se termine. L'analyseur courant est lu dans la pile de la JVM (`jcmd <pid> Thread.print`),
# Ghidra n'écrivant aucun avancement dans son journal en mode sans interface.
#
# Usage : scripts/dev/ghidra/watch_analysis.sh <pid_java> [intervalle_s]
# JCMD peut pointer vers un autre JDK (défaut : Temurin 25).

pid="$1"
interval="${2:-60}"
jcmd="${JCMD:-/c/Program Files/Eclipse Adoptium/jdk-25.0.2.10-hotspot/bin/jcmd.exe}"
last=""
last_report=$(date +%s)

stats() {
  powershell -NoProfile -Command "\$p = Get-Process -Id $pid -ErrorAction SilentlyContinue; if (\$p) { '{0} min CPU, {1} Go' -f [math]::Round(\$p.CPU/60,1), [math]::Round(\$p.WorkingSet64/1GB,1) }" 2>/dev/null | tr -d '\r'
}

while true; do
  if ! tasklist //FI "PID eq $pid" 2>/dev/null | grep -q "$pid"; then
    echo "$(date +%H:%M) JVM $pid terminée (dernier analyseur : ${last:-inconnu})"
    exit 0
  fi
  # Cadre qui suit `AnalysisScheduler.runAnalyzer` : l'analyseur en cours d'exécution.
  current=$("$jcmd" "$pid" Thread.print 2>/dev/null \
    | grep -B1 "AnalysisScheduler.runAnalyzer" \
    | grep -oE "ghidra\.[A-Za-z0-9_.]*\.[A-Z][A-Za-z0-9_]*(Analyzer|Cmd)[A-Za-z0-9_]*" \
    | head -1 | sed 's/.*\.//')
  [ -z "$current" ] && current="(hors analyseur)"
  now=$(date +%s)
  if [ "$current" != "$last" ]; then
    echo "$(date +%H:%M) analyseur : $current, $(stats)"
    last="$current"
    last_report=$now
  elif [ $((now - last_report)) -ge 900 ]; then
    echo "$(date +%H:%M) toujours dans $current, $(stats)"
    last_report=$now
  fi
  sleep "$interval"
done
