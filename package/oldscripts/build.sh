#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "${SCRIPT_DIR}/common.sh"

usage() {
    echo "usage: build.sh <image> <version> <debname>"
    echo ""
    echo "  image:   A docker image name, for example 'ubuntu:jammy'"
    echo "  version: A version string, for example 'v0.0.1'"
    echo "  debname: The name of the deb file to produce, for example 'kfx-jammy.deb'"
    fail
}

# Build xen intermediate docker image
build_xen_intermediate() {
    IMAGE="${1}"
    HASH="${2}"
    OUTPUT_IMGFILE="package/cache/xen-intermediate-${IMAGE}-${HASH}.tar.gz"

    log_info "Building image ${IMAGE} with Xen hash ${HASH} to ${OUTPUT_IMGFILE}"

    if ! docker build --build-arg "IMAGE=${IMAGE}" \
        -f "${DOCKERFILE_XEN}" -t xen-intermediate '.' &> "${LOGFILE}"; then
        log_error "Xen intermediate image build failed:"
        fail
    fi

    log_info "Removing old Xen intermediate image"

    rm -f package/cache/xen-intermediate-*.tar.gz

    log_info "Saving new Xen intermediate image"

    docker save xen-intermediate | gzip -c > "${OUTPUT_IMGFILE}"

    if [ ! -f "${OUTPUT_IMGFILE}" ]; then
        log_error "Output image file ${OUTPUT_IMGFILE} not found"
        rm package/cache/xen-intermediate-*.tar.gz
        fail
    fi
}

if [[ $# != 3 ]]; then
    usage
fi

echo "Starting build. Log written to ${LOGFILE}"

XEN_HASH=$(git ls-files -s xen | cut -f2 '-d ')
OUTPUT_IMGFILE="package/cache/xen-intermediate-${IMAGE}-${XEN_HASH}.tar.gz"

mkdir -p package/cache
mkdir -p package/log

if [ ! -f "${OUTPUT_IMGFILE}" ]; then
    build_xen_intermediate "${IMAGE}" "${XEN_HASH}"
else
    log_info "Loading cached Xen intermediate image"

    if ! docker load < "${OUTPUT_IMGFILE}"; then
        log_error "Failed to load Xen intermediate image"
        fail
    fi
fi

log_info Building final image "${IMAGE}" 

if ! docker build -f package/Dockerfile-final \
    -t deb-build --build-arg "IMAGE=${IMAGE}" '.' &> "${LOGFILE}"; then
    log_error "Failed to build final KF/x dockerfile"
    fail

fi

if ! docker run -v "$(pwd)/package/out:/out" deb-build ./package/mkdeb "${IMAGE}"; then
    log_error "Failed to run deb package build"
    fail
fi

exit 0
