#!/bin/bash

# Configure script to go boom immediately if a command fails.
set -e
set -o pipefail
set -x

# Constants.
IMAGE_TAG="synthea"
SEED=42

# Use GNU getopt to parse the options passed to this script.
TEMP=`getopt \
	p:t: \
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
		-t )
			targetDirectory="$2"; shift 2 ;;
		-- ) shift; break ;;
		* ) break ;;
	esac
done

# Verify that all required options were specified.
if [[ -z "${populationSize}" ]]; then >&2 echo 'The -p option for desired population size is required.'; exit 1; fi
if [[ -z "${targetDirectory}" ]]; then >&2 echo 'The -t option for the target directory is required.'; exit 1; fi

# Create the target directory, if needed.
if [[ ! -d "${targetDirectory}" ]]; then mkdir "${targetDirectory}"; fi

# This setting enables proper `docker build` caching, which is particularly important for CI systems.
export DOCKER_BUILDKIT=1

# Build the Docker image for Synthea.
docker build --file ./Dockerfile.synthea -t "${IMAGE_TAG}" --cache-from docker.pkg.github.com/karlmdavis/fhir-benchmarks/synthea --build-arg BUILDKIT_INLINE_CACHE=1 .

# Run Synthea, with the specified options.
docker run \
  --rm \
  --mount source="${targetDirectory}",target="/synthea/target/",type=bind \
  --user "$(id -u)" \
  --entrypoint '/bin/bash' \
  "${IMAGE_TAG}" \
  -x -c 'id && ls -lan / && ls -lan /synthea'
docker run \
  --rm \
  --mount source="${targetDirectory}",target="/synthea/target/",type=bind \
  --user "$(id -u)" \
  "${IMAGE_TAG}" \
  -s "${SEED}" \
  -cs "${SEED}" \
  -p "${populationSize}"
