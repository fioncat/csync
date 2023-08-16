package main

import (
	"context"
	"fmt"
	"path"
	"time"

	"github.com/dustin/go-humanize"
	"github.com/redis/go-redis/v9"
	"github.com/sirupsen/logrus"
)

type Redis struct {
	key string

	client *redis.Client

	formatter Formatter
}

func NewRedis() (*Redis, error) {
	key := fmt.Sprintf("/csync/%s", GetConfig().Name)
	client := redis.NewClient(GetConfig().Redis.Options())
	err := client.Ping(context.Background()).Err()
	if err != nil {
		return nil, fmt.Errorf("Ping redis server error: %w", err)
	}

	var formatter Formatter
	if GetConfig().Password == "" {
		formatter = &base64Formatter{}
	} else {
		formatter, err = newPasswordFormatter(GetConfig().Password)
		if err != nil {
			return nil, fmt.Errorf("Init password formatter error: %w", err)
		}
	}

	return &Redis{
		key:       key,
		client:    client,
		formatter: formatter,
	}, nil
}

func (r *Redis) Send(frame *DataFrame) error {
	ctx := context.Background()
	data := frame.Encode()

	start := time.Now()
	message, err := r.formatter.Encode(data)
	if err != nil {
		return fmt.Errorf("Formatter encode data frame: %w", err)
	}
	frame.LogEntry.Infof("Formatter encode data done, took %v", time.Since(start))

	start = time.Now()
	err = r.client.Publish(ctx, r.key, message).Err()
	if err != nil {
		return fmt.Errorf("Redis publish error: %w", err)
	}

	frame.LogEntry.Infof("Publish redis done, took %v", time.Since(start))
	return nil
}

func (r *Redis) Sub() <-chan *DataFrame {
	ch := make(chan *DataFrame, 1000)
	go r.sub(ch)
	return ch
}

func (r *Redis) sub(ch chan *DataFrame) {
	watchNames := GetConfig().Watch
	watchKeys := make([]string, len(watchNames))
	for i, name := range watchNames {
		watchKeys[i] = fmt.Sprintf("/csync/%s", name)
	}

	logrus.Infof("Begin to sub redis: %v", watchKeys)

	ctx := context.Background()
	sub := r.client.Subscribe(ctx, watchKeys...)
	defer sub.Close()

	for msg := range sub.Channel() {
		channel := msg.Channel
		name := path.Base(channel)
		entry := logrus.New().WithField("From", name)

		start := time.Now()
		data, err := r.formatter.Decode(msg.Payload)
		if err != nil {
			entry.Errorf("Format received data error: %v", err)
			continue
		}

		entry.Infof("Formatter decode %s data done, took %v", humanize.Bytes(uint64(len(data))), time.Since(start))

		start = time.Now()
		frame, err := DecodeDataFrame(data, name, entry)
		if err != nil {
			entry.Errorf("Decode data frame error: %v", err)
			continue
		}
		frame.LogEntry.Infof("Decode data done, took %v", time.Since(start))

		ch <- frame
	}
}
