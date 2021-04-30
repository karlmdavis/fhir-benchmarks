#!/bin/bash

set -e
set -o pipefail

curl \
  'https://raw.githubusercontent.com/FirelyTeam/spark/r4/master/.docker/docker-compose.example.yml' \
  > docker-compose.yml
docker-compose up
