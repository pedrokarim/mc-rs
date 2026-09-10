"""
Simple UDP proxy to bypass UWP loopback restriction.
Listens on 0.0.0.0:19133 and forwards to 127.0.0.1:19132.
Minecraft connects to this proxy instead of directly to the server.
"""
import socket
import threading
import sys

LISTEN_PORT = 19133
SERVER_HOST = "127.0.0.1"
SERVER_PORT = 19132

def main():
    proxy = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    proxy.bind(("0.0.0.0", LISTEN_PORT))
    print(f"UDP proxy listening on 0.0.0.0:{LISTEN_PORT} -> {SERVER_HOST}:{SERVER_PORT}")

    # Map: server sees proxy_port -> we remember which client to reply to
    client_addr = None
    server_addr = (SERVER_HOST, SERVER_PORT)

    # Create a separate socket for talking to the server
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server_sock.setblocking(False)

    proxy.setblocking(False)

    import select
    while True:
        readable, _, _ = select.select([proxy, server_sock], [], [], 1.0)
        for sock in readable:
            if sock is proxy:
                # Packet from Minecraft client
                data, addr = proxy.recvfrom(65535)
                client_addr = addr
                server_sock.sendto(data, server_addr)
            elif sock is server_sock:
                # Packet from our MC-RS server
                data, _ = server_sock.recvfrom(65535)
                if client_addr:
                    proxy.sendto(data, client_addr)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nProxy stopped.")
