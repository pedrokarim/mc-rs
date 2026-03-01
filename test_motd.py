#!/usr/bin/env python3
"""
Test MOTD (Unconnected Pong) from a Bedrock server.

Usage:
    python test_motd.py                    # test localhost:19132
    python test_motd.py 127.0.0.1 19132    # test specific host/port
    python test_motd.py --compare           # compare mc-rs (19132) vs BDS (19133)
"""

import socket
import struct
import sys
import time

RAKNET_MAGIC = bytes([
    0x00, 0xFF, 0xFF, 0x00,
    0xFE, 0xFE, 0xFE, 0xFE,
    0xFD, 0xFD, 0xFD, 0xFD,
    0x12, 0x34, 0x56, 0x78,
])

UNCONNECTED_PING = 0x01
UNCONNECTED_PONG = 0x1C

MOTD_FIELDS = [
    "edition",        # MCPE
    "server_name",    # Server Name
    "protocol",       # 924
    "game_version",   # 1.26.2
    "online",         # 0
    "max_players",    # 20
    "server_guid",    # 12345
    "world_name",     # world
    "gamemode",       # Survival
    "gamemode_num",   # 1
    "ipv4_port",      # 19132
    "ipv6_port",      # 19133
    "editor_mode",    # 0
]


def build_ping() -> bytes:
    """Build an Unconnected Ping packet."""
    timestamp = int(time.time() * 1000) & 0x7FFFFFFFFFFFFFFF
    client_guid = 0x1234567890ABCDEF
    return struct.pack(">BqBq", UNCONNECTED_PING, timestamp, 0, 0)[0:1] + \
           struct.pack(">q", timestamp) + RAKNET_MAGIC + struct.pack(">q", client_guid)


def parse_pong(data: bytes) -> dict:
    """Parse an Unconnected Pong packet, return MOTD fields."""
    if len(data) < 35:
        raise ValueError(f"Pong too short: {len(data)} bytes")

    packet_id = data[0]
    if packet_id != UNCONNECTED_PONG:
        raise ValueError(f"Not a Pong: 0x{packet_id:02X}")

    # Skip: packet_id(1) + timestamp(8) + server_guid(8) + magic(16) = 33 bytes
    offset = 33
    # String: u16_be length + data
    str_len = struct.unpack(">H", data[offset:offset+2])[0]
    offset += 2
    motd_str = data[offset:offset+str_len].decode("utf-8", errors="replace")

    parts = motd_str.split(";")
    result = {}
    for i, field_name in enumerate(MOTD_FIELDS):
        result[field_name] = parts[i] if i < len(parts) else ""
    result["_raw"] = motd_str
    result["_field_count"] = len(parts)
    return result


def ping_server(host: str, port: int, timeout: float = 3.0) -> dict:
    """Send Unconnected Ping and return parsed MOTD."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(timeout)
    try:
        ping = build_ping()
        sock.sendto(ping, (host, port))
        data, addr = sock.recvfrom(4096)
        return parse_pong(data)
    finally:
        sock.close()


def print_motd(label: str, motd: dict):
    """Pretty-print MOTD fields."""
    print(f"\n{'=' * 60}")
    print(f"  {label}")
    print(f"{'=' * 60}")
    for field in MOTD_FIELDS:
        val = motd.get(field, "")
        print(f"  {field:<15} : {val}")
    print(f"  {'field_count':<15} : {motd['_field_count']}")
    print(f"  {'raw':<15} : {motd['_raw']}")
    print()


def compare_motds(motd_a: dict, motd_b: dict, label_a: str, label_b: str):
    """Compare two MOTDs field by field."""
    print(f"\n{'=' * 60}")
    print(f"  COMPARISON: {label_a} vs {label_b}")
    print(f"{'=' * 60}")

    all_ok = True
    # Skip server_name, server_guid, online — those will differ
    skip = {"server_name", "server_guid", "online"}

    for field in MOTD_FIELDS:
        a = motd_a.get(field, "")
        b = motd_b.get(field, "")
        if field in skip:
            status = "SKIP"
        elif a == b:
            status = "OK"
        else:
            status = "DIFF"
            all_ok = False
        print(f"  {field:<15} : {a:<25} | {b:<25} [{status}]")

    fc_a = motd_a["_field_count"]
    fc_b = motd_b["_field_count"]
    fc_status = "OK" if fc_a == fc_b else "DIFF"
    print(f"  {'field_count':<15} : {str(fc_a):<25} | {str(fc_b):<25} [{fc_status}]")

    if all_ok and fc_a == fc_b:
        print("\n  RESULT: ALL MATCHING")
    else:
        print("\n  RESULT: DIFFERENCES FOUND")


def main():
    args = sys.argv[1:]

    if "--compare" in args:
        # Compare mc-rs (19132) vs BDS (19133)
        mcrs_port = 19132
        bds_port = 19133
        host = "127.0.0.1"

        print(f"Pinging mc-rs at {host}:{mcrs_port}...")
        try:
            motd_mcrs = ping_server(host, mcrs_port)
            print_motd(f"mc-rs ({host}:{mcrs_port})", motd_mcrs)
        except Exception as e:
            print(f"  ERROR: mc-rs not responding: {e}")
            motd_mcrs = None

        print(f"Pinging BDS at {host}:{bds_port}...")
        try:
            motd_bds = ping_server(host, bds_port)
            print_motd(f"BDS ({host}:{bds_port})", motd_bds)
        except Exception as e:
            print(f"  ERROR: BDS not responding: {e}")
            motd_bds = None

        if motd_mcrs and motd_bds:
            compare_motds(motd_mcrs, motd_bds, "mc-rs", "BDS")
    else:
        # Single server test
        host = args[0] if len(args) >= 1 else "127.0.0.1"
        port = int(args[1]) if len(args) >= 2 else 19132

        print(f"Pinging {host}:{port}...")
        try:
            motd = ping_server(host, port)
            print_motd(f"{host}:{port}", motd)
        except socket.timeout:
            print(f"  ERROR: No response (timeout). Is the server running?")
            sys.exit(1)
        except Exception as e:
            print(f"  ERROR: {e}")
            sys.exit(1)


if __name__ == "__main__":
    main()
