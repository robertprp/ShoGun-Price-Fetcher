#!/usr/bin/env bash

set -e

# Check if $1 is missing
if [ -z "$CONFIG_PATH" ]; then
    echo "Error: CONFIG_PATH environment variable is not set."
    exit 1
fi

echo "entry.sh: Config $CONFIG_PATH"
/app/grafana-shogun --config $CONFIG_PATH --log-level info