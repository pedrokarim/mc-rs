# Framing des Paquets Bedrock (Protocol 924)

Source: BedrockProtocol/DataPacket.php

## Couches d'encapsulation

```
UDP Datagram
  └─ RakNet Frame
       └─ 0xFE prefix (game packet marker, géré par RakNet)
            └─ Batch (compressé ou non)
                 └─ [sub_packet_1][sub_packet_2]...
                      └─ Chaque sub_packet: VarUInt32(len) + data
                           └─ data: VarUInt32(header) + body
```

## Header VarUInt32 — Structure (BedrockProtocol)

```
Bits 0-9   : Packet ID (10 bits, max 1023)
Bits 10-11 : Sender Sub-Client ID (0-3, normalement 0)
Bits 12-13 : Recipient Sub-Client ID (0-3, normalement 0)
```

### Formules

**Encodage:**
```
header = (packet_id & 0x3FF) | (sender_sub << 10) | (recipient_sub << 12)
```
Pour le cas normal (pas de split-screen): `header = packet_id`

**Décodage:**
```
packet_id      = header & 0x3FF
sender_sub     = (header >> 10) & 0x03
recipient_sub  = (header >> 12) & 0x03
```

### ATTENTION
- PAS de `<< 2` ni `>> 2` !
- L'ancienne approche `packet_id << 2` est INCORRECTE pour BedrockProtocol
- Chaque paquet envoyé/reçu avec le mauvais framing sera silencieusement ignoré par le client

## Compression

### Avant NetworkSettings
- Pas de compression
- Le batch est juste les sub-packets concaténés

### Après NetworkSettings
- Premier byte = algorithme de compression
  - `0x00` = Zlib (Deflate)
  - `0x01` = Snappy
  - `0xFF` = Pas de compression

### NetworkSettings (envoyé par le serveur)
```
compressionThreshold: u16 BE = 1
compressionAlgorithm: u16 BE = 1 (Snappy)
clientThrottleEnabled: bool = false
clientThrottleThreshold: u8 = 0
clientThrottleScalar: f32 BE = 0.0
```

## Batch Format

```
[VarUInt32(sub_pkt_1_len)][sub_pkt_1_data]
[VarUInt32(sub_pkt_2_len)][sub_pkt_2_data]
...
```

Chaque `sub_pkt_data`:
```
[VarUInt32(header)][body_bytes]
```

## Envoi via RakNet

```
0xFE + compressed_batch_data
```

Reliability: `ReliableOrdered`, channel 0.
