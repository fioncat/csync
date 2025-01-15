#!/bin/bash

csync-server() {
  echo "WARNING: Starting test csync server"
  cargo build
  if [[ $? -ne 0 ]]; then
    return 1
  fi
  ./target/debug/csync server --config-path ./testdata/config/server --data-path ./testdata/data/server
}

csync-test() {
  echo "WARNING: Running test csync"
  cargo build
  if [[ $? -ne 0 ]]; then
    return 1
  fi
  ./target/debug/csync put role wheel -r "texts,images,files:*" --config-path ./testdata/config/server --data-path ./testdata/data/server
  ./target/debug/csync put user fioncat -r wheel -p "test123" --config-path ./testdata/config/server --data-path ./testdata/data/server
  echo "==============================================="
  ./target/debug/csync $@ --config-path ./testdata/config/client --data-path ./testdata/data/client
}
