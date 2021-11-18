#!/bin/bash

set -e
set -o pipefail

# Grab the directory that this script lives in.
scriptDirectory="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Run whatever docker-compose command was specified against the downloaded docker-compose.yml file.
docker-compose \
  --file "${scriptDirectory}/docker-compose.yml" \
  "$@"