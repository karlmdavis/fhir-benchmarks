#!/bin/bash

# Configure script to go boom immediately if a command fails.
set -e
set -o pipefail

# Constants.
IMAGE_TAG="benchmarks/synthea"
SEED=42

# Use GNU getopt to parse the options passed to this script.
TEMP=`getopt \
	p: \
	$*`
if [ $? != 0 ] ; then echo "Terminating." >&2 ; exit 1 ; fi

# Note the quotes around `$TEMP': they are essential!
eval set -- "$TEMP"

# Parse the getopt results.
populationSize=""
while true; do
	case "$1" in
		-p )
			populationSize="$2"; shift 2 ;;
		-- ) shift; break ;;
		* ) break ;;
	esac
done

# Verify that all required options were specified.
if [[ -z "${populationSize}" ]]; then >&2 echo 'The -p option for desired population size is required.'; exit 1; fi

# This setting enables proper `docker build` caching, which is particularly important for CI systems.
export DOCKER_BUILDKIT=1

# Build the Docker image for Synthea.
docker build --file ./Dockerfile.synthea --build-arg UID="$(id -u)" --build-arg GID="$(id -g)" -t "${IMAGE_TAG}" .

# Run Synthea, with the specified options.
docker run --rm --mount source="$(pwd)/target/",target="/synthea/target/",type=bind "${IMAGE_TAG}" \
  -s "${SEED}" \
  -cs "${SEED}" \
  -p "${populationSize}"