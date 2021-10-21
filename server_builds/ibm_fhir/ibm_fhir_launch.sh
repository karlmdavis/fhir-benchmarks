#!/bin/bash

set -e
set -o pipefail

# Grab the directory that this script lives in.
scriptDirectory="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Copy the config files we need out of the image.
docker cp \
  $(docker create --rm ibmcom/ibm-fhir-server:4.8.3):/opt/ol/wlp/usr/servers/defaultServer/config/default \
  "${scriptDirectory}/volumes/config/"
mv \
  "${scriptDirectory}/volumes/config/default/fhir-server-config-postgresql.json" \
  "${scriptDirectory}/volumes/config/default/fhir-server-config.json"
docker cp \
  $(docker create --rm ibmcom/ibm-fhir-server:4.8.3):/opt/ol/wlp/usr/servers/defaultServer/configDropins/disabled/datasource-postgresql.xml \
  "${scriptDirectory}/volumes/overrides/datasource.xml"

# Launch the containers specified in docker-compose.yml.
COMPOSE_DOCKER_CLI_BUILD=1 \
  DOCKER_BUILDKIT=1 \
  docker-compose up --detach