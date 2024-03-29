#!/bin/bash

BIN_PATH=./target/release/csync
if [[ ! -f $BIN_PATH ]]; then
	cargo build --release --locked
fi

while true; do
	$BIN_PATH watch
	echo ">>>> Csync exited, restart"
	sleep 1
done
