package main

import (
	"bytes"
	"crypto/sha256"
	"fmt"
	"time"

	"github.com/dustin/go-humanize"
	"github.com/sirupsen/logrus"
	"golang.design/x/clipboard"
)

type Clipboard struct {
	reids *Redis

	hash []byte
}

func NewClipboard() (*Clipboard, error) {
	err := clipboard.Init()
	if err != nil {
		return nil, fmt.Errorf("Init system clipboard driver error: %v", err)
	}

	redis, err := NewRedis()
	if err != nil {
		return nil, err
	}

	return &Clipboard{reids: redis}, nil
}

func (c *Clipboard) recv(frame *DataFrame) {
	start := time.Now()
	hashArray := sha256.Sum256(frame.Data)
	hash := hashArray[:]
	if bytes.Equal(hash, c.hash) {
		frame.LogEntry.Infof("Recv same clipboard data, skip writing clipboard")
		return
	}
	c.hash = hash

	frame.LogEntry.Infof("")
}

func (c *Clipboard) send(dataType DataFrameType, data []byte) {
	if len(data) == 0 {
		logrus.Infof("Clipboard return empty data, skip sending")
		return
	}

	// TODO: validate max size exceeded

	dataSize := humanize.Bytes(uint64(len(data)))
	entry := logrus.StandardLogger().WithFields(logrus.Fields{
		"DataSize": dataSize,
		"Type":     dataType,
	})

	start := time.Now()
	hashArray := sha256.Sum256(data)
	hash := hashArray[:]
	if bytes.Equal(hash, c.hash) {
		entry.Infof("Clipboard data not changed, skip sending")
		return
	}
	c.hash = hash

	entry.Infof("Calculate sha256 for data done, took %v", time.Since(start))

	frame := &DataFrame{
		Type:     dataType,
		Data:     data,
		LogEntry: entry,
	}
	err := c.reids.Send(frame)
	if err != nil {
		frame.LogEntry.Errorf("Send data frame to redis error: %v", err)
		return
	}
}
