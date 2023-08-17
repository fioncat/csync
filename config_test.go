package main

import (
	"bytes"
	"fmt"
	"testing"

	"gopkg.in/yaml.v3"
)

func TestConfig(t *testing.T) {
	cfg, err := ReadConfig()
	if err != nil {
		t.Fatal(err)
	}

	var buf bytes.Buffer
	encoder := yaml.NewEncoder(&buf)
	encoder.SetIndent(2)
	err = encoder.Encode(cfg)
	if err != nil {
		t.Fatal(err)
	}

	fmt.Println(buf.String())
}
