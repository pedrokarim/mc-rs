#!/usr/bin/env python3
"""
MC-RS Benchmark Suite
Compare mc-rs (Rust) vs PocketMine-MP (PHP) vs BDS (Mojang C++)

Usage: python benchmarks/bench.py
"""

import json
import math
import os
import shutil
import socket
import statistics
import struct
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path

# ══════════════════════════════════════════════════
#  Constants
# ══════════════════════════════════════════════════

PROJECT_ROOT = Path(__file__).resolve().parent.parent
RAKNET_MAGIC = bytes([
    0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78,
])
STARTUP_TIMEOUT = 120  # seconds
IDLE_SAMPLE_DURATION = 10  # seconds
PING_COUNT = 100
PING_DELAY = 0.01  # 10ms between pings


# ══════════════════════════════════════════════════
#  Data structures
# ══════════════════════════════════════════════════

@dataclass
class ServerConfig:
    name: str
    display_name: str
    working_dir: Path
    launch_cmd: list[str]
    ready_pattern: str
    process_name: str
    config_file: str          # relative to working_dir
    port_key: str
    port: int
    config_format: str        # "toml" or "properties"
    build_cmd: list[str] | None = None
    build_cwd: Path | None = None
    env_extra: dict = field(default_factory=dict)
    extra_config: dict = field(default_factory=dict)


@dataclass
class BenchmarkResult:
    server_name: str
    startup_time_ms: float = 0.0
    memory_mb: float = 0.0
    cpu_percent: float = 0.0
    ping_min_ms: float = 0.0
    ping_avg_ms: float = 0.0
    ping_max_ms: float = 0.0
    ping_p99_ms: float = 0.0
    ping_success: int = 0
    ping_total: int = 0
    error: str | None = None


# ══════════════════════════════════════════════════
#  Server configurations
# ══════════════════════════════════════════════════

SERVERS = [
    ServerConfig(
        name="mc-rs",
        display_name="mc-rs (Rust)",
        working_dir=PROJECT_ROOT,
        launch_cmd=[str(PROJECT_ROOT / "target" / "release" / "mc-rs-server.exe")],
        ready_pattern="Server listening on",
        process_name="mc-rs-server",
        config_file="server.toml",
        port_key="port",
        port=19132,
        config_format="toml",
        build_cmd=["cargo", "build", "--release"],
        build_cwd=PROJECT_ROOT,
        env_extra={"RUST_LOG": "info"},
    ),
    ServerConfig(
        name="pmmp",
        display_name="PocketMine-MP",
        working_dir=PROJECT_ROOT / ".reference" / "pocketmine-test",
        launch_cmd=[
            str(PROJECT_ROOT / ".reference" / "pocketmine-test" / "bin" / "php" / "php.exe"),
            str(PROJECT_ROOT / ".reference" / "pocketmine-test" / "PocketMine-MP.phar"),
        ],
        ready_pattern="Done (",
        process_name="php",
        config_file="server.properties",
        port_key="server-port",
        port=19133,
        config_format="properties",
    ),
    ServerConfig(
        name="bds",
        display_name="BDS (Mojang)",
        working_dir=PROJECT_ROOT / ".reference" / "bds" / "server",
        launch_cmd=[str(PROJECT_ROOT / ".reference" / "bds" / "server" / "bedrock_server.exe")],
        ready_pattern="Server started",
        process_name="bedrock_server",
        config_file="server.properties",
        port_key="server-port",
        port=19134,
        config_format="properties",
        extra_config={"enable-lan-visibility": "false"},
    ),
]


# ══════════════════════════════════════════════════
#  Logging helpers
# ══════════════════════════════════════════════════

COLORS = {
    "reset": "\033[0m",
    "bold": "\033[1m",
    "green": "\033[32m",
    "yellow": "\033[33m",
    "red": "\033[31m",
    "cyan": "\033[36m",
    "dim": "\033[2m",
}


def log(server_name: str, msg: str, color: str = "cyan"):
    c = COLORS.get(color, "")
    r = COLORS["reset"]
    tag = f"[{server_name:<15}]"
    print(f"  {c}{tag}{r} {msg}", flush=True)


def banner():
    print()
    print(f"  {COLORS['bold']}{'=' * 60}{COLORS['reset']}")
    print(f"  {COLORS['bold']}  MC-RS Benchmark Suite{COLORS['reset']}")
    print(f"  {COLORS['dim']}  mc-rs vs PocketMine-MP vs BDS (Mojang){COLORS['reset']}")
    print(f"  {COLORS['bold']}{'=' * 60}{COLORS['reset']}")
    print()


# ══════════════════════════════════════════════════
#  Config file manipulation (backup / modify / restore)
# ══════════════════════════════════════════════════

def backup_config(config: ServerConfig):
    """Backup the server config file."""
    src = config.working_dir / config.config_file
    dst = config.working_dir / (config.config_file + ".bench.bak")
    if src.exists():
        shutil.copy2(src, dst)


def restore_config(config: ServerConfig):
    """Restore the server config file from backup."""
    src = config.working_dir / (config.config_file + ".bench.bak")
    dst = config.working_dir / config.config_file
    if src.exists():
        shutil.copy2(src, dst)
        src.unlink()


def set_port(config: ServerConfig):
    """Modify the config file to use the benchmark port."""
    filepath = config.working_dir / config.config_file
    if not filepath.exists():
        return

    lines = filepath.read_text(encoding="utf-8").splitlines()
    new_lines = []

    if config.config_format == "toml":
        for line in lines:
            stripped = line.strip()
            if stripped.startswith(f"{config.port_key}") and "=" in stripped:
                key_part = stripped.split("=")[0].strip()
                if key_part == config.port_key:
                    line = f"{config.port_key} = {config.port}"
            new_lines.append(line)
    elif config.config_format == "properties":
        for line in lines:
            stripped = line.strip()
            if stripped.startswith(f"{config.port_key}="):
                line = f"{config.port_key}={config.port}"
            # Apply extra config changes
            for key, value in config.extra_config.items():
                if stripped.startswith(f"{key}="):
                    line = f"{key}={value}"
            new_lines.append(line)

    filepath.write_text("\n".join(new_lines) + "\n", encoding="utf-8")


# ══════════════════════════════════════════════════
#  Process management
# ══════════════════════════════════════════════════

def kill_process(process_name: str):
    """Force-kill a process by name using PowerShell."""
    subprocess.run(
        ["powershell", "-Command",
         f'Get-Process {process_name} -ErrorAction SilentlyContinue | Stop-Process -Force'],
        capture_output=True, timeout=10,
    )
    time.sleep(0.5)


def start_server(config: ServerConfig) -> tuple[subprocess.Popen, float]:
    """Start a server and wait for the ready pattern. Returns (process, startup_time_ms)."""
    env = os.environ.copy()
    env.update(config.env_extra)

    t0 = time.perf_counter()

    proc = subprocess.Popen(
        config.launch_cmd,
        cwd=str(config.working_dir),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        stdin=subprocess.PIPE,
        env=env,
        bufsize=0,
    )

    # Read stdout in a thread to avoid blocking
    output_lines = []
    ready_event = threading.Event()

    def reader():
        try:
            for raw_line in iter(proc.stdout.readline, b""):
                try:
                    line = raw_line.decode("utf-8", errors="replace").rstrip()
                except Exception:
                    line = str(raw_line)
                output_lines.append(line)
                if config.ready_pattern in line:
                    ready_event.set()
        except Exception:
            pass

    t = threading.Thread(target=reader, daemon=True)
    t.start()

    if not ready_event.wait(timeout=STARTUP_TIMEOUT):
        # Dump last lines for debugging
        log(config.name, "TIMEOUT — last output:", "red")
        for line in output_lines[-10:]:
            log(config.name, f"  | {line}", "dim")
        raise TimeoutError(f"Server did not become ready within {STARTUP_TIMEOUT}s")

    startup_ms = (time.perf_counter() - t0) * 1000
    return proc, startup_ms


def stop_server(config: ServerConfig, proc: subprocess.Popen | None = None):
    """Stop the server gracefully, then force-kill."""
    if proc and proc.poll() is None:
        # Try graceful shutdown via stdin
        try:
            proc.stdin.write(b"stop\n")
            proc.stdin.flush()
        except Exception:
            pass
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass

    kill_process(config.process_name)


# ══════════════════════════════════════════════════
#  Measurement functions
# ══════════════════════════════════════════════════

def get_pid(config: ServerConfig) -> int | None:
    """Get PID of the server process."""
    try:
        result = subprocess.run(
            ["powershell", "-Command",
             f'(Get-Process {config.process_name} -ErrorAction SilentlyContinue | Select-Object -First 1).Id'],
            capture_output=True, text=True, timeout=10,
        )
        pid_str = result.stdout.strip()
        if pid_str:
            return int(pid_str)
    except Exception:
        pass
    return None


def measure_memory(pid: int, duration: int = IDLE_SAMPLE_DURATION) -> float:
    """Measure average memory (MB) over duration seconds using PowerShell."""
    ps_script = f"""
$samples = @()
for ($i = 0; $i -lt {duration}; $i++) {{
    try {{
        $mem = (Get-Process -Id {pid} -ErrorAction Stop).WorkingSet64
        $samples += $mem
    }} catch {{ }}
    Start-Sleep 1
}}
$samples -join ","
"""
    try:
        result = subprocess.run(
            ["powershell", "-Command", ps_script],
            capture_output=True, text=True, timeout=duration + 15,
        )
        values = [int(v) for v in result.stdout.strip().split(",") if v.strip()]
        if values:
            return statistics.mean(values) / (1024 * 1024)  # bytes -> MB
    except Exception:
        pass
    return 0.0


def measure_cpu(pid: int, duration: int = IDLE_SAMPLE_DURATION) -> float:
    """Measure CPU usage (%) over duration seconds."""
    ps_script = f"""
$p = Get-Process -Id {pid} -ErrorAction Stop
$cpu0 = $p.TotalProcessorTime.TotalMilliseconds
$t0 = [System.Diagnostics.Stopwatch]::GetTimestamp()
Start-Sleep {duration}
$p.Refresh()
$cpu1 = $p.TotalProcessorTime.TotalMilliseconds
$t1 = [System.Diagnostics.Stopwatch]::GetTimestamp()
$freq = [System.Diagnostics.Stopwatch]::Frequency
$wall_ms = ($t1 - $t0) / $freq * 1000
$cores = [Environment]::ProcessorCount
$pct = ($cpu1 - $cpu0) / ($wall_ms * $cores) * 100
[math]::Round($pct, 2)
"""
    try:
        result = subprocess.run(
            ["powershell", "-Command", ps_script],
            capture_output=True, text=True, timeout=duration + 15,
        )
        val = result.stdout.strip()
        # Handle comma as decimal separator (French locale)
        val = val.replace(",", ".")
        if val:
            return float(val)
    except Exception:
        pass
    return 0.0


def measure_ping_latency(port: int, count: int = PING_COUNT) -> dict:
    """Send RakNet UnconnectedPing packets and measure RTT."""
    client_guid = 12345678
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(2.0)

    rtts = []
    success = 0

    for i in range(count):
        # Build UnconnectedPing packet
        send_time_ns = time.time_ns() // 1_000_000  # ms since epoch
        ping = struct.pack(">B q", 0x01, send_time_ns)
        ping += RAKNET_MAGIC
        ping += struct.pack(">q", client_guid)

        try:
            t0 = time.perf_counter()
            sock.sendto(ping, ("127.0.0.1", port))
            data, _ = sock.recvfrom(4096)
            t1 = time.perf_counter()

            if data[0] == 0x1C:  # UnconnectedPong
                rtt_ms = (t1 - t0) * 1000
                rtts.append(rtt_ms)
                success += 1
        except socket.timeout:
            pass
        except Exception:
            pass

        time.sleep(PING_DELAY)

    sock.close()

    if not rtts:
        return {"min": 0, "avg": 0, "max": 0, "p99": 0, "success": 0, "total": count}

    rtts.sort()
    p99_idx = max(0, math.ceil(len(rtts) * 0.99) - 1)

    return {
        "min": round(min(rtts), 3),
        "avg": round(statistics.mean(rtts), 3),
        "max": round(max(rtts), 3),
        "p99": round(rtts[p99_idx], 3),
        "success": success,
        "total": count,
    }


# ══════════════════════════════════════════════════
#  Output formatting
# ══════════════════════════════════════════════════

def print_results_table(results: list[BenchmarkResult]):
    """Print a nice ASCII comparison table."""
    print()
    print(f"  {COLORS['bold']}{'=' * 80}{COLORS['reset']}")
    print(f"  {COLORS['bold']}  RESULTS{COLORS['reset']}")
    print(f"  {COLORS['bold']}{'=' * 80}{COLORS['reset']}")
    print()

    # Header
    h = f"  {'Server':<18} {'Startup':>10} {'Memory':>10} {'CPU':>8} {'Ping min':>9} {'Ping avg':>9} {'Ping max':>9} {'Ping p99':>9}"
    print(f"  {COLORS['bold']}{h.strip()}{COLORS['reset']}")
    print(f"  {'-' * 78}")

    for r in results:
        if r.error:
            print(f"  {r.server_name:<18} {COLORS['red']}ERROR: {r.error}{COLORS['reset']}")
        else:
            startup = f"{r.startup_time_ms:.0f} ms"
            memory = f"{r.memory_mb:.1f} MB"
            cpu = f"{r.cpu_percent:.2f} %"
            p_min = f"{r.ping_min_ms:.3f}"
            p_avg = f"{r.ping_avg_ms:.3f}"
            p_max = f"{r.ping_max_ms:.3f}"
            p_p99 = f"{r.ping_p99_ms:.3f}"
            print(f"  {r.server_name:<18} {startup:>10} {memory:>10} {cpu:>8} {p_min:>9} {p_avg:>9} {p_max:>9} {p_p99:>9}")

    print()
    print(f"  {COLORS['dim']}Ping values in milliseconds. {PING_COUNT} pings per server.{COLORS['reset']}")
    print()


def save_results(results: list[BenchmarkResult]):
    """Save results to JSON."""
    output_path = PROJECT_ROOT / "benchmarks" / "results.json"
    data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "config": {
            "startup_timeout_s": STARTUP_TIMEOUT,
            "idle_sample_duration_s": IDLE_SAMPLE_DURATION,
            "ping_count": PING_COUNT,
        },
        "results": [asdict(r) for r in results],
    }
    output_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
    log("bench", f"Results saved to {output_path}", "green")


def save_html(results: list[BenchmarkResult]):
    """Save results as a visual HTML report."""
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    valid = [r for r in results if r.error is None]

    # Find max values for bar scaling
    max_startup = max((r.startup_time_ms for r in valid), default=1)
    max_memory = max((r.memory_mb for r in valid), default=1)
    max_ping = max((r.ping_avg_ms for r in valid), default=1)

    # Server colors
    server_colors = {
        "mc-rs (Rust)": "#e8590c",
        "PocketMine-MP": "#7048e8",
        "BDS (Mojang)": "#2f9e44",
    }

    def bar(value: float, max_val: float, color: str) -> str:
        pct = (value / max_val * 100) if max_val > 0 else 0
        return (
            f'<div class="bar-bg">'
            f'<div class="bar-fill" style="width:{pct:.1f}%;background:{color}"></div>'
            f'</div>'
        )

    def winner_class(value: float, all_values: list[float], lower_is_better: bool = True) -> str:
        if not all_values:
            return ""
        best = min(all_values) if lower_is_better else max(all_values)
        return " winner" if value == best else ""

    # Collect values for winner detection
    startups = [r.startup_time_ms for r in valid]
    memories = [r.memory_mb for r in valid]
    pings_avg = [r.ping_avg_ms for r in valid]

    # Build server cards HTML
    cards_html = ""
    for r in results:
        color = server_colors.get(r.server_name, "#868e96")
        if r.error:
            cards_html += f"""
            <div class="card error">
                <div class="card-header" style="border-color:{color}">
                    <span class="server-dot" style="background:{color}"></span>
                    <h2>{r.server_name}</h2>
                </div>
                <div class="card-body">
                    <p class="error-msg">Error: {r.error}</p>
                </div>
            </div>"""
            continue

        cards_html += f"""
            <div class="card">
                <div class="card-header" style="border-color:{color}">
                    <span class="server-dot" style="background:{color}"></span>
                    <h2>{r.server_name}</h2>
                </div>
                <div class="card-body">
                    <div class="metric">
                        <div class="metric-header">
                            <span class="metric-label">Startup Time</span>
                            <span class="metric-value{winner_class(r.startup_time_ms, startups)}">{r.startup_time_ms:.0f} ms</span>
                        </div>
                        {bar(r.startup_time_ms, max_startup, color)}
                    </div>
                    <div class="metric">
                        <div class="metric-header">
                            <span class="metric-label">Memory (idle)</span>
                            <span class="metric-value{winner_class(r.memory_mb, memories)}">{r.memory_mb:.1f} MB</span>
                        </div>
                        {bar(r.memory_mb, max_memory, color)}
                    </div>
                    <div class="metric">
                        <div class="metric-header">
                            <span class="metric-label">CPU (idle)</span>
                            <span class="metric-value">{r.cpu_percent:.2f} %</span>
                        </div>
                    </div>
                    <div class="metric">
                        <div class="metric-header">
                            <span class="metric-label">Ping avg</span>
                            <span class="metric-value{winner_class(r.ping_avg_ms, pings_avg)}">{r.ping_avg_ms:.3f} ms</span>
                        </div>
                        {bar(r.ping_avg_ms, max_ping, color)}
                    </div>
                    <div class="ping-details">
                        <span>min {r.ping_min_ms:.3f}</span>
                        <span>p99 {r.ping_p99_ms:.3f}</span>
                        <span>max {r.ping_max_ms:.3f}</span>
                        <span>{r.ping_success}/{r.ping_total} ok</span>
                    </div>
                </div>
            </div>"""

    # Build comparison table rows
    table_rows = ""
    for r in valid:
        color = server_colors.get(r.server_name, "#868e96")
        table_rows += f"""
                <tr>
                    <td><span class="server-dot" style="background:{color}"></span> {r.server_name}</td>
                    <td class="{('best' if r.startup_time_ms == min(startups) else '')}">{r.startup_time_ms:.0f} ms</td>
                    <td class="{('best' if r.memory_mb == min(memories) else '')}">{r.memory_mb:.1f} MB</td>
                    <td>{r.cpu_percent:.2f} %</td>
                    <td>{r.ping_min_ms:.3f}</td>
                    <td class="{('best' if r.ping_avg_ms == min(pings_avg) else '')}">{r.ping_avg_ms:.3f}</td>
                    <td>{r.ping_max_ms:.3f}</td>
                    <td>{r.ping_p99_ms:.3f}</td>
                </tr>"""

    # Compute speedup ratios vs slowest
    ratios_html = ""
    if len(valid) >= 2:
        best_startup = min(startups)
        best_mem = min(memories)
        best_ping = min(pings_avg)
        for r in valid:
            color = server_colors.get(r.server_name, "#868e96")
            s_ratio = r.startup_time_ms / best_startup if best_startup > 0 else 0
            m_ratio = r.memory_mb / best_mem if best_mem > 0 else 0
            p_ratio = r.ping_avg_ms / best_ping if best_ping > 0 else 0
            ratios_html += f"""
                <tr>
                    <td><span class="server-dot" style="background:{color}"></span> {r.server_name}</td>
                    <td>{'1x (best)' if s_ratio <= 1.01 else f'{s_ratio:.1f}x slower'}</td>
                    <td>{'1x (best)' if m_ratio <= 1.01 else f'{m_ratio:.1f}x more'}</td>
                    <td>{'1x (best)' if p_ratio <= 1.01 else f'{p_ratio:.1f}x slower'}</td>
                </tr>"""

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>MC-RS Benchmark Results</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0f0f0f;
            color: #e0e0e0;
            padding: 2rem;
            line-height: 1.5;
        }}
        .container {{ max-width: 1100px; margin: 0 auto; }}
        h1 {{
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 0.25rem;
        }}
        .subtitle {{
            color: #888;
            font-size: 0.95rem;
            margin-bottom: 2rem;
        }}
        .cards {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
            gap: 1.25rem;
            margin-bottom: 2.5rem;
        }}
        .card {{
            background: #1a1a1a;
            border-radius: 12px;
            overflow: hidden;
        }}
        .card.error {{ opacity: 0.5; }}
        .card-header {{
            padding: 1rem 1.25rem;
            border-top: 3px solid;
            display: flex;
            align-items: center;
            gap: 0.6rem;
        }}
        .card-header h2 {{ font-size: 1.1rem; font-weight: 600; }}
        .server-dot {{
            width: 10px;
            height: 10px;
            border-radius: 50%;
            display: inline-block;
            flex-shrink: 0;
        }}
        .card-body {{ padding: 0 1.25rem 1.25rem; }}
        .metric {{ margin-bottom: 1rem; }}
        .metric-header {{
            display: flex;
            justify-content: space-between;
            align-items: baseline;
            margin-bottom: 0.3rem;
        }}
        .metric-label {{ color: #999; font-size: 0.85rem; }}
        .metric-value {{ font-size: 1.1rem; font-weight: 600; font-variant-numeric: tabular-nums; }}
        .metric-value.winner {{ color: #51cf66; }}
        .bar-bg {{
            height: 6px;
            background: #2a2a2a;
            border-radius: 3px;
            overflow: hidden;
        }}
        .bar-fill {{
            height: 100%;
            border-radius: 3px;
            transition: width 0.6s ease;
        }}
        .ping-details {{
            display: flex;
            gap: 1rem;
            font-size: 0.8rem;
            color: #666;
            margin-top: -0.5rem;
        }}
        .error-msg {{ color: #ff6b6b; font-style: italic; }}

        /* Comparison table */
        .section-title {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 1rem;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid #2a2a2a;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-bottom: 2.5rem;
            font-variant-numeric: tabular-nums;
        }}
        th {{
            text-align: left;
            padding: 0.6rem 0.75rem;
            font-size: 0.8rem;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            color: #888;
            border-bottom: 1px solid #2a2a2a;
        }}
        td {{
            padding: 0.6rem 0.75rem;
            border-bottom: 1px solid #1a1a1a;
            font-size: 0.95rem;
        }}
        td.best {{
            color: #51cf66;
            font-weight: 600;
        }}
        tr:hover td {{ background: #1a1a1a; }}

        .footer {{
            text-align: center;
            color: #555;
            font-size: 0.8rem;
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid #1a1a1a;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>MC-RS Benchmark</h1>
        <p class="subtitle">mc-rs (Rust) vs PocketMine-MP (PHP) vs BDS (Mojang C++) &mdash; {timestamp}</p>

        <div class="cards">
            {cards_html}
        </div>

        <h3 class="section-title">Comparison</h3>
        <table>
            <thead>
                <tr>
                    <th>Server</th>
                    <th>Startup</th>
                    <th>Memory</th>
                    <th>CPU</th>
                    <th>Ping min</th>
                    <th>Ping avg</th>
                    <th>Ping max</th>
                    <th>Ping p99</th>
                </tr>
            </thead>
            <tbody>
                {table_rows}
            </tbody>
        </table>

        <h3 class="section-title">Relative Performance</h3>
        <table>
            <thead>
                <tr>
                    <th>Server</th>
                    <th>Startup</th>
                    <th>Memory</th>
                    <th>Ping</th>
                </tr>
            </thead>
            <tbody>
                {ratios_html}
            </tbody>
        </table>

        <div class="footer">
            Generated by MC-RS Benchmark Suite &mdash; {PING_COUNT} pings per server, {IDLE_SAMPLE_DURATION}s idle sampling
        </div>
    </div>
</body>
</html>"""

    output_path = PROJECT_ROOT / "benchmarks" / "results.html"
    output_path.write_text(html, encoding="utf-8")
    log("bench", f"HTML report saved to {output_path}", "green")


# ══════════════════════════════════════════════════
#  Main orchestrator
# ══════════════════════════════════════════════════

def benchmark_server(config: ServerConfig) -> BenchmarkResult:
    """Run all benchmarks for a single server."""
    result = BenchmarkResult(server_name=config.display_name)

    # Kill any leftover process
    log(config.name, "Cleaning up leftover processes...", "dim")
    kill_process(config.process_name)

    # Backup and modify config
    log(config.name, f"Setting port to {config.port}...", "dim")
    backup_config(config)
    set_port(config)

    proc = None
    try:
        # Build if needed
        if config.build_cmd:
            log(config.name, "Building...", "yellow")
            build_result = subprocess.run(
                config.build_cmd,
                cwd=str(config.build_cwd or config.working_dir),
                capture_output=True, text=True, timeout=300,
            )
            if build_result.returncode != 0:
                log(config.name, "Build FAILED:", "red")
                for line in build_result.stderr.splitlines()[-10:]:
                    log(config.name, f"  | {line}", "dim")
                result.error = "Build failed"
                return result

        # Start server
        log(config.name, "Starting server...", "yellow")
        proc, startup_ms = start_server(config)
        result.startup_time_ms = round(startup_ms, 1)
        log(config.name, f"Ready in {startup_ms:.0f} ms", "green")

        # Get PID
        pid = get_pid(config)
        if not pid:
            # Fallback: use Popen pid
            pid = proc.pid

        # Measure memory
        log(config.name, f"Measuring memory ({IDLE_SAMPLE_DURATION}s)...", "yellow")
        result.memory_mb = round(measure_memory(pid), 1)
        log(config.name, f"Memory: {result.memory_mb:.1f} MB", "green")

        # Measure CPU
        log(config.name, f"Measuring CPU ({IDLE_SAMPLE_DURATION}s)...", "yellow")
        result.cpu_percent = round(measure_cpu(pid), 2)
        log(config.name, f"CPU: {result.cpu_percent:.2f} %", "green")

        # Measure ping latency
        log(config.name, f"Ping latency ({PING_COUNT} pings on port {config.port})...", "yellow")
        pings = measure_ping_latency(config.port)
        result.ping_min_ms = pings["min"]
        result.ping_avg_ms = pings["avg"]
        result.ping_max_ms = pings["max"]
        result.ping_p99_ms = pings["p99"]
        result.ping_success = pings["success"]
        result.ping_total = pings["total"]
        log(config.name, f"Ping: {pings['avg']:.3f} ms avg ({pings['success']}/{pings['total']} ok)", "green")

    except TimeoutError as e:
        result.error = str(e)
        log(config.name, f"ERROR: {e}", "red")
    except Exception as e:
        result.error = str(e)
        log(config.name, f"ERROR: {e}", "red")
    finally:
        # Stop server
        log(config.name, "Stopping server...", "yellow")
        stop_server(config, proc)
        log(config.name, "Done.", "green")

        # Restore config
        restore_config(config)

    return result


def main():
    banner()

    results = []
    for config in SERVERS:
        print()
        log(config.name, f"--- Benchmarking {config.display_name} ---", "bold")
        result = benchmark_server(config)
        results.append(result)
        print()

    print_results_table(results)
    save_results(results)
    save_html(results)


if __name__ == "__main__":
    main()
