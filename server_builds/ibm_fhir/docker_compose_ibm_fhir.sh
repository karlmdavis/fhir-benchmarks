#!/bin/bash

set -e
set -o pipefail

# Specify the version of the IBM FHIR server to use.
version="4.9.2"

# Grab the directory that this script lives in.
scriptDirectory="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Download the resources required to run the server via Docker Compose.
if [[ ! -f "${scriptDirectory}/${version}.tar.gz" ]]; then
  wget --directory-prefix="${scriptDirectory}" "https://github.com/IBM/FHIR/archive/${version}.tar.gz"
fi
if [[ ! -d "${scriptDirectory}/FHIR-${version}" ]]; then
  tar --directory "${scriptDirectory}" -x -z -f "${scriptDirectory}/${version}.tar.gz"
fi

# Run whatever docker-compose command was specified against the downloaded docker-compose.yml file.
COMPOSE_DOCKER_CLI_BUILD=1 \
  DOCKER_BUILDKIT=1 \
  docker-compose --file "${scriptDirectory}/FHIR-${version}/demo/docker-compose.yml" "$@"
