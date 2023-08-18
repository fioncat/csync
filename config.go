package main

import (
	"errors"
	"fmt"
	"os"
	"path"
	"reflect"
	"strconv"
	"sync"
	"time"

	"github.com/redis/go-redis/v9"
	"gopkg.in/yaml.v3"
)

type Config struct {
	Name     string      `yaml:"name"`
	Password string      `yaml:"password"`
	Watch    []string    `yaml:"watch"`
	Redis    RedisConfig `yaml:"redis"`
}

type RedisConfig struct {
	Host string `yaml:"host" default:"127.0.0.1"`
	Port int    `yaml:"port" default:"6379"`

	User     string `yaml:"user"`
	Password string `yaml:"password"`

	Timeout string `yaml:"timeout" default:"10s"`

	timeout time.Duration
}

func (c *Config) normalize() error {
	if c.Name == "" {
		host, err := os.Hostname()
		if err != nil {
			return fmt.Errorf("Get hostname: %w", err)
		}
		c.Name = host
		if c.Name == "" {
			return errors.New("Host name for your machine is empty")
		}
	}

	redis := c.Redis
	if redis.Host == "" {
		return errors.New("Redis host could not be empty")
	}
	if redis.Port <= 0 {
		return fmt.Errorf("Invalid redis port %d", redis.Port)
	}

	if redis.Timeout == "" {
		redis.timeout = time.Second * 10
	} else {
		timeout, err := time.ParseDuration(redis.Timeout)
		if err != nil {
			return fmt.Errorf("Parse config error, invalid timeout %q", redis.Timeout)
		}
		mills := timeout.Milliseconds()
		if mills <= 100 {
			return fmt.Errorf("Config redis timeout is too small, should bigger than 100ms")
		}
		redis.timeout = timeout
	}

	return nil
}

func (r *RedisConfig) Options() *redis.Options {
	addr := fmt.Sprintf("%s:%d", r.Host, r.Port)
	return &redis.Options{
		Addr: addr,

		Username: r.User,
		Password: r.Password,

		DialTimeout:  r.timeout,
		ReadTimeout:  r.timeout,
		WriteTimeout: r.timeout,
	}
}

func ReadConfig() (*Config, error) {
	path, err := configPath()
	if err != nil {
		return nil, fmt.Errorf("Get config path: %w", err)
	}

	var cfg Config
	if path != "" {
		file, err := os.Open(path)
		if err != nil {
			return nil, fmt.Errorf("Open config file: %w", err)
		}
		defer file.Close()
		decoder := yaml.NewDecoder(file)
		err = decoder.Decode(&cfg)
		if err != nil {
			return nil, fmt.Errorf("Decode config file: %w", err)
		}
	}

	setDefault(&cfg)
	setDefault(&cfg.Redis)

	err = cfg.normalize()
	if err != nil {
		return nil, fmt.Errorf("Normalize config: %w", err)
	}

	return &cfg, nil
}

func configPath() (string, error) {
	configPath := os.Getenv("RSYNC_CONFIG")
	if configPath != "" {
		return configPath, nil
	}
	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("Get home dir: %w", err)
	}
	dir = path.Join(dir, ".config", "csync")
	meta, err := os.Stat(dir)
	if err != nil {
		if os.IsNotExist(err) {
			return "", nil
		}
		return "", fmt.Errorf("Stat config dir: %w", err)
	}
	if !meta.IsDir() {
		return "", nil
	}

	entries, err := os.ReadDir(dir)
	if err != nil {
		return "", fmt.Errorf("Read config dir: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		name := entry.Name()
		switch name {
		case "config.yaml", "config.yml":
			return path.Join(dir, name), nil
		}
	}
	return "", nil
}

func GetMetaDir() (string, error) {
	dir := os.Getenv("CSYNC_LOCAL")
	if dir == "" {
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("Get user home dir: %w", err)
		}
		dir = path.Join(homeDir, ".local", "share", "csync")
	}

	stat, err := os.Stat(dir)
	if err != nil {
		if os.IsNotExist(err) {
			err = os.MkdirAll(dir, os.ModePerm)
			if err != nil {
				return "", fmt.Errorf("Mkdir for csync local: %w", err)
			}
			return dir, nil
		}
		return "", fmt.Errorf("Stat csync local error: %w", err)
	}

	if !stat.IsDir() {
		return "", fmt.Errorf("Local %s is not a directory", dir)
	}

	return dir, nil
}

func setDefault(v any) {
	value := reflect.ValueOf(v).Elem()
	structType := value.Type()
	n := value.NumField()
	for i := 0; i < n; i++ {
		fieldType := structType.Field(i)
		defaultValue := fieldType.Tag.Get("default")
		if defaultValue == "" {
			continue
		}

		field := value.Field(i)
		switch field.Kind() {
		case reflect.String:
			if field.String() != "" {
				continue
			}
			field.SetString(defaultValue)

		case reflect.Int:
			if field.Int() > 0 {
				continue
			}
			intValue, err := strconv.ParseInt(defaultValue, 10, 64)
			if err != nil {
				panic(err)
			}
			field.SetInt(intValue)
		}
	}
}

var (
	configInstance *Config
	configOnce     sync.Once
)

func GetConfig() *Config {
	configOnce.Do(func() {
		cfg, err := ReadConfig()
		if err != nil {
			ErrorExit(err)
		}
		configInstance = cfg
	})
	return configInstance
}
