package main

import (
	"crypto/aes"
	"crypto/cipher"
	crand "crypto/rand"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"

	"github.com/dustin/go-humanize"
	"github.com/sirupsen/logrus"
)

type DataFrameType (uint16)

const (
	DataFrameImage DataFrameType = 1
	DataFrameText  DataFrameType = 2
)

type DataFrame struct {
	Type DataFrameType

	From string
	Data []byte

	LogEntry *logrus.Entry
}

func NewDataFrameText(text string) *DataFrame {
	return &DataFrame{
		Type:     DataFrameText,
		Data:     []byte(text),
		LogEntry: logrus.NewEntry(logrus.New()),
	}
}

func NewDataFrameImage(data []byte) *DataFrame {
	return &DataFrame{
		Type:     DataFrameImage,
		Data:     data,
		LogEntry: logrus.NewEntry(logrus.New()),
	}
}

func (f *DataFrame) Encode() []byte {
	buf := make([]byte, 2)
	binary.BigEndian.PutUint16(buf, uint16(f.Type))
	return append(buf, f.Data...)
}

func DecodeDataFrame(data []byte, from string, entry *logrus.Entry) (*DataFrame, error) {
	if len(data) < 2 {
		return nil, errors.New("Data frame too small")
	}

	typeData, data := data[:2], data[2:]
	dataType := DataFrameType(binary.BigEndian.Uint16(typeData))

	switch dataType {
	case DataFrameImage, DataFrameText:

	default:
		return nil, fmt.Errorf("Data frame invalid type %d", dataType)
	}
	entry = entry.WithFields(logrus.Fields{
		"Type":     dataType,
		"DataSize": humanize.Bytes(uint64(len(data))),
	})

	return &DataFrame{
		Type:     dataType,
		From:     from,
		Data:     data,
		LogEntry: entry,
	}, nil
}

var (
	ErrIncorrectPassword = errors.New("incorrect password")
	ErrInvalidHexFormat  = errors.New("Message is not hex format")
)

type Formatter interface {
	Encode(data []byte) (string, error)
	Decode(message string) ([]byte, error)
}

type base64Formatter struct{}

func (f *base64Formatter) Encode(data []byte) (string, error) {
	return hex.EncodeToString(data), nil
}

func (f *base64Formatter) Decode(message string) ([]byte, error) {
	data, err := hex.DecodeString(message)
	if err != nil {
		return nil, ErrInvalidHexFormat
	}
	return data, nil
}

type passwordFormatter struct {
	aead cipher.AEAD
}

func newPasswordFormatter(password string) (Formatter, error) {
	sum := sha256.Sum256([]byte(password))
	key := sum[:32]

	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, fmt.Errorf("Init AES cipher: %w", err)
	}

	// gcm or Galois/Counter Mode, is a mode of operation for symmetric key
	// cryptographic block ciphers.
	// See: https://en.wikipedia.org/wiki/Galois/Counter_Mode
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, fmt.Errorf("Init GCM: %w", err)
	}

	return &passwordFormatter{aead: gcm}, nil
}

func (f *passwordFormatter) Encode(data []byte) (string, error) {
	nonce := make([]byte, f.aead.NonceSize())

	_, err := io.ReadFull(crand.Reader, nonce)
	if err != nil {
		return "", fmt.Errorf("Generate password random sequence: %w", err)
	}

	result := f.aead.Seal(nonce, nonce, data, nil)
	return hex.EncodeToString(result), nil
}

func (f *passwordFormatter) Decode(message string) ([]byte, error) {
	data, err := hex.DecodeString(message)
	if err != nil {
		return nil, ErrInvalidHexFormat
	}

	nonceSize := f.aead.NonceSize()
	if len(data) < nonceSize {
		return nil, ErrIncorrectPassword
	}
	var nonce []byte

	nonce, data = data[:nonceSize], data[nonceSize:]
	result, err := f.aead.Open(nil, nonce, data, nil)
	if err != nil {
		return nil, ErrIncorrectPassword
	}

	return result, nil
}
