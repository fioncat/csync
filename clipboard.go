package main

import (
	"bytes"
	"context"
	"crypto/sha256"
	"fmt"
	"time"

	"github.com/dustin/go-humanize"
	"github.com/sirupsen/logrus"
	"golang.design/x/clipboard"
)

type Clipboard struct {
	redis *Redis

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

	return &Clipboard{redis: redis}, nil
}

func (c *Clipboard) Run() {
	ctx := context.Background()
	textChan := clipboard.Watch(ctx, clipboard.FmtText)
	imageChan := clipboard.Watch(ctx, clipboard.FmtImage)

	if len(GetConfig().Watch) == 0 || GetConfig().Clipboard.ReadOnly {
		logrus.Info("Start to listen clipboard")
		for {
			select {
			case textData := <-textChan:
				c.publishRedis(DataFrameText, textData)

			case imageData := <-imageChan:
				c.publishRedis(DataFrameImage, imageData)
			}
		}
	}

	subChan := c.redis.Sub()
	if GetConfig().Clipboard.WriteOnly {
		logrus.Info("Start to subscribe redis")
		for frame := range subChan {
			c.writeClipboard(frame, textChan, imageChan)
		}
		return
	}

	logrus.Info("Start to listen and receive clipboard")
	for {
		select {
		case frame := <-subChan:
			c.writeClipboard(frame, textChan, imageChan)

		case textData := <-textChan:
			c.publishRedis(DataFrameText, textData)

		case imageData := <-imageChan:
			c.publishRedis(DataFrameImage, imageData)
		}
	}
}

func (c *Clipboard) writeClipboard(frame *DataFrame, textChan <-chan []byte, imageChan <-chan []byte) {
	start := time.Now()
	hashArray := sha256.Sum256(frame.Data)
	hash := hashArray[:]
	if bytes.Equal(hash, c.hash) {
		frame.LogEntry.Infof("Recv same clipboard data, skip writing clipboard")
		return
	}
	c.hash = hash

	frame.LogEntry.Infof("Recv calculate sha256 for data done, took %v", time.Since(start))

	start = time.Now()
	switch frame.Type {
	case DataFrameText:
		clipboard.Write(clipboard.FmtText, frame.Data)
	case DataFrameImage:
		clipboard.Write(clipboard.FmtImage, frame.Data)
	}
	switch frame.Type {
	case DataFrameText:
		<-textChan

	case DataFrameImage:
		<-imageChan
	}
	frame.LogEntry.Infof("Write data to clipboard done, took %v", time.Since(start))
}

func (c *Clipboard) publishRedis(dataType DataFrameType, data []byte) {
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
	err := c.redis.Send(frame)
	if err != nil {
		frame.LogEntry.Errorf("Send data frame to redis error: %v", err)
		return
	}
}
