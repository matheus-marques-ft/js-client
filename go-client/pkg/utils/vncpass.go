package vncpass

import (
	"bytes"
	"crypto/des"
	"os"
	"path/filepath"
)

// Fixed VNC raw key
var originalKey = []byte{0x17, 0x52, 0x6b, 0x09, 0x33, 0x51, 0x6e, 0x4b}

// Bit-reverse a byte (used for the TigerVNC encryption key)
func reverseBits(b byte) byte {
	var rev byte
	for i := 0; i < 8; i++ {
		rev = (rev << 1) | (b & 1)
		b >>= 1
	}
	return rev
}

// Build the DES key required by VNC
func generateVNCKey() []byte {
	key := make([]byte, len(originalKey))
	for i, b := range originalKey {
		key[i] = reverseBits(b)
	}
	return key
}

// Convert the plaintext password to 8 bytes
func padPassword(pw string) []byte {
	p := []byte(pw)
	if len(p) > 8 {
		return p[:8]
	}
	return append(p, bytes.Repeat([]byte{0}, 8-len(p))...)
}

// Generate the .vncpass file and return its path
func GenerateVNCPasswordFile(password string) (string, error) {
	// Get the ~/.config/jumpserver-client path
	configDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	currentPath := filepath.Join(configDir, "jumpserver-client")

	// Make sure the directory exists
	err = os.MkdirAll(currentPath, os.ModePerm)
	if err != nil {
		return "", err
	}

	// Random filename
	filename := ".vncpaxx"
	outputPath := filepath.Join(currentPath, filename)

	// Encryption
	key := generateVNCKey()
	block, err := des.NewCipher(key)
	if err != nil {
		return "", err
	}
	plaintext := padPassword(password)
	ciphertext := make([]byte, 8)
	block.Encrypt(ciphertext, plaintext)

	// Write the file
	err = os.WriteFile(outputPath, ciphertext, os.ModePerm)
	if err != nil {
		return "", err
	}

	return outputPath, nil
}
