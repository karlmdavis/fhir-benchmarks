#!/bin/bash

set -e
set -o pipefail

# Specify the version of the Spark server to use.
version="v1.5.10-r4"  # Published 2021-10-19.

# Grab the directory that this script lives in.
scriptDirectory="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Download the resources required to run the server via Docker Compose.
curl -L -s -S \
  "https://github.com/FirelyTeam/spark/raw/${version}/.docker/docker-compose.example.yml" \
  > "${scriptDirectory}/docker-compose.yml"

# Replace the 'r4-latest' tag with the desired version.
#
# Note: right now, the Spark `.docker/docker-compose.example.yml` file always points to the `r4-latest`
# Docker image, and so needs to be edited to ensure that our benchmarks are running against a stable target.
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed \
    -i '' \
    -e "s/spark:r4-latest/spark:${version}/g" \
    "${scriptDirectory}/docker-compose.yml"
  sed \
    -i '' \
    -e "s/mongo:r4-latest/mongo:${version}/g" \
    "${scriptDirectory}/docker-compose.yml"
else
  sed \
    -i \
    -e "s/spark:r4-latest/spark:${version}/g" \
    "${scriptDirectory}/docker-compose.yml"
  sed \
    -i \
    -e "s/mongo:r4-latest/mongo:${version}/g" \
    "${scriptDirectory}/docker-compose.yml"
fi

# Run whatever docker-compose command was specified against the downloaded docker-compose.yml file.
docker-compose \
  --file "${scriptDirectory}/docker-compose.yml" \
  "$@"