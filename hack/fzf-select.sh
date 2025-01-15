#!/usr/bin/env bash

set -e

chosen=$(csync read text -l | fzf)
echo -n "$chosen" | csync read text --from-selected-stdin | csync cb -wd
