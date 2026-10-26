"""Fait générer et enregistrer à BDS une zone de chunks, puis l'arrête proprement.

Lance `bedrock_server.exe`, ajoute une `tickingarea` sur le rectangle de blocs donné (100 chunks au plus), attend,
force l'écriture (`save hold`, `save query` jusqu'à « Data saved », `save resume`), retire la zone et arrête
le serveur. Journal dans `<dossier BDS>/bds-stdout6.log`. BDS n'enregistre pas forcément toute la zone dans le
délai : vérifier les chunks présents dans la base (`world_block_classes.py extract`). Lire ensuite une COPIE de
la base. Usage dans dev-docs/14-WORLD-GENERATION.md (« Parité avec BDS »).

Usage : python scripts/dev/bds_load_area.py <dossier BDS> <x1> <z1> <x2> <z2> [attente_s]
"""
import subprocess
import sys
import threading
import time
from pathlib import Path

bds_dir = Path(sys.argv[1])
x1, z1, x2, z2 = map(int, sys.argv[2:6])
wait = int(sys.argv[6]) if len(sys.argv) > 6 else 40
log = open(bds_dir / 'bds-stdout6.log', 'w', encoding='utf-8')
lines: list[str] = []

proc = subprocess.Popen([str(bds_dir / 'bedrock_server.exe')], cwd=bds_dir, stdin=subprocess.PIPE,
                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding='utf-8',
                        errors='replace', bufsize=1)


def reader():
    for line in proc.stdout:
        lines.append(line)
        log.write(line)
        log.flush()


threading.Thread(target=reader, daemon=True).start()


def send(cmd):
    print('>', cmd, flush=True)
    proc.stdin.write(cmd + '\n')
    proc.stdin.flush()


def wait_for(text, timeout):
    start = time.time()
    seen = len(lines)
    while time.time() - start < timeout:
        for line in lines[seen:]:
            if text in line:
                return line
        time.sleep(0.5)
    return None


if not wait_for('Server started', 180):
    print('BDS ne démarre pas'); proc.kill(); sys.exit(1)
send(f'tickingarea add {x1} 0 {z1} {x2} 0 {z2} mcrs_area')
print(wait_for('', 5) and ''.join(lines[-3:]), flush=True)
time.sleep(wait)
send('save hold')
time.sleep(3)
for _ in range(60):
    send('save query')
    if wait_for('Data saved', 5):
        break
send('save resume')
time.sleep(2)
send('tickingarea remove mcrs_area')
time.sleep(2)
send('stop')
proc.wait(timeout=120)
print('BDS arrêté, code', proc.returncode)
print(''.join(l for l in lines if 'tickingarea' in l.lower() or 'Ticking' in l or 'saved' in l.lower()))
