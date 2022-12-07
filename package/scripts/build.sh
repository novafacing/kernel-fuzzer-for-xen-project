#!/bin/bash

# Package build script
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "${SCRIPT_DIR}/../.." > /dev/null

IMAGE="${1}"
KFX_VERSION="${2}"
OUT_DIR="${3}"

if [ -z "${IMAGE}" ] || [ -z "${KFX_VERSION}" ]; then
    echo "Usage: ${0} <image> <kfx-version> [out-dir]"
    exit 1
fi

if [ -z "${OUT_DIR}" ]; then
    OUT_DIR="${SCRIPT_DIR}/artifacts/"
fi
          
CODENAME="$(echo ${IMAGE} | awk -F':' '{print $2}' | head -n 1)"

mkdir -p "${OUT_DIR}"

TAG="kfx-builder-${IMAGE}"

# Build the image
docker build -t "${TAG}" -f "${SCRIPT_DIR}/../docker/Dockerfile" \
    --build-arg IMAGE="${IMAGE}" \
    --build-arg KFX_VERSION="${KFX_VERSION}" \
    "."

# Create a temporary container
CONTAINER=$(docker create "${TAG}")

# Copy the deb packages to the host
docker cp "${CONTAINER}:/debs/" "${OUT_DIR}/"

# Remove the temporary container
docker rm "${CONTAINER}"

popd