package main

import (
	"encoding/binary"
	"fmt"
	"io"
	"os"
)

// record est un paquet enregistré : instant depuis le début de la session, identifiant, charge utile.
type record struct {
	ms      uint64
	id      uint32
	payload []byte
}

func writeRecords(path string, records []record) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	for _, r := range records {
		var head [16]byte
		binary.LittleEndian.PutUint64(head[0:], r.ms)
		binary.LittleEndian.PutUint32(head[8:], r.id)
		binary.LittleEndian.PutUint32(head[12:], uint32(len(r.payload)))
		if _, err := f.Write(head[:]); err != nil {
			return err
		}
		if _, err := f.Write(r.payload); err != nil {
			return err
		}
	}
	return nil
}

func readRecords(path string) ([]record, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var records []record
	for i := 0; i < len(data); {
		if len(data)-i < 16 {
			return records, fmt.Errorf("enregistrement tronqué à l'octet %d", i)
		}
		r := record{
			ms: binary.LittleEndian.Uint64(data[i:]),
			id: binary.LittleEndian.Uint32(data[i+8:]),
		}
		n := int(binary.LittleEndian.Uint32(data[i+12:]))
		i += 16
		if len(data)-i < n {
			return records, io.ErrUnexpectedEOF
		}
		r.payload = data[i : i+n]
		i += n
		records = append(records, r)
	}
	return records, nil
}
