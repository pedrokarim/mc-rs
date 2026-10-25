package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"sync"

	"github.com/sandertv/gophertunnel/minecraft/auth"
	"golang.org/x/oauth2"
)

// defaultTokenPath place le jeton dans le dossier de configuration de l'utilisateur, jamais dans le dépôt.
func defaultTokenPath() string {
	dir, err := os.UserConfigDir()
	if err != nil {
		return "bds-capture-token.json"
	}
	return filepath.Join(dir, "mc-rs", "bds-capture-token.json")
}

// savingTokenSource rafraîchit le jeton Microsoft et l'enregistre à chaque changement, pour ne demander
// la connexion (code d'appareil) qu'une seule fois.
type savingTokenSource struct {
	mu   sync.Mutex
	src  oauth2.TokenSource
	path string
	last string
}

func (s *savingTokenSource) Token() (*oauth2.Token, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	t, err := s.src.Token()
	if err != nil {
		return nil, err
	}
	if t.RefreshToken != s.last {
		s.last = t.RefreshToken
		if data, err := json.Marshal(t); err == nil {
			os.MkdirAll(filepath.Dir(s.path), 0o700)
			if err := os.WriteFile(s.path, data, 0o600); err != nil {
				log.Printf("jeton non enregistré : %v", err)
			}
		}
	}
	return t, nil
}

// tokenSource charge le jeton enregistré dans path, ou lance la connexion par code d'appareil
// (https://www.microsoft.com/link).
func tokenSource(path string) (oauth2.TokenSource, error) {
	var token *oauth2.Token
	if data, err := os.ReadFile(path); err == nil {
		token = new(oauth2.Token)
		if err := json.Unmarshal(data, token); err != nil {
			return nil, err
		}
	} else {
		t, err := auth.RequestLiveToken()
		if err != nil {
			return nil, err
		}
		token = t
	}
	src := &savingTokenSource{src: auth.RefreshTokenSource(token), path: path}
	if _, err := src.Token(); err != nil {
		return nil, err
	}
	return src, nil
}
