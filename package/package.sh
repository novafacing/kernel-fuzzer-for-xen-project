#!/bin/bash

# Print usage information
usage() {
    echo "usage: package.sh <IMAGE> [OUTDIR]"
    echo "  IMAGE:  The name of the image to build on, or 'all'."
    echo "    Options: 'all', 'jammy', 'focal', 'buster', 'bullseye', 'bionic'"
    echo "  OUTDIR: Optional path to output directory for .deb files."
    echo "    Default: creates a new temp directory."
    exit 1
}

# Populate a .env file with proxy information to pass to act
populate_env_file() {
    ENV_FILE="${1}"
    echo "Running with ENV_FILE=${ENV_FILE}"

    if [ -z "${HTTP_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        PACKAGE_HTTP_PROXY=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config PACKAGE_HTTP_PROXY=${PACKAGE_HTTP_PROXY}"
    fi
    echo "PACKAGE_HTTP_PROXY=${PACKAGE_HTTP_PROXY}" >> "${ENV_FILE}"

    if [ -z "${HTTPS_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        PACKAGE_HTTPS_PROXY=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config PACKAGE_HTTPS_PROXY=${PACKAGE_HTTPS_PROXY}"
    fi
    echo "PACKAGE_HTTPS_PROXY=${PACKAGE_HTTPS_PROXY}" >> "${ENV_FILE}"

    if [ -z "${NO_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        PACKAGE_NO_PROXY=$(grep noProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config PACKAGE_NO_PROXY=${PACKAGE_NO_PROXY}"
    fi
    echo "PACKAGE_NO_PROXY=${PACKAGE_NO_PROXY}" >> "${ENV_FILE}"

    cat "${ENV_FILE}"
}

WORKFLOW="${1}"
OUTDIR="${2}"

if [ -z "${WORKFLOW}" ]; then
    usage
fi

if [ -z "${OUTDIR}" ]; then
    OUTDIR=$(mktemp -p /tmp -d kfx-artifacts-XXXXXX)
fi

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "${SCRIPT_DIR}/.." > /dev/null

git submodule update --init --recursive

WORKFLOW_FILE="${SCRIPT_DIR}/workflows/package-${WORKFLOW}.yml"

echo "Running with WORKFLOW_FILE=${WORKFLOW_FILE}"

ENV_FILE=$(mktemp -p /tmp .env.XXXXXXXX)

populate_env_file "${ENV_FILE}"

act -W "${WORKFLOW_FILE}" --env-file="${ENV_FILE}" --artifact-server-path="${OUTDIR}"

find "${OUTDIR}" -type f -name '*.gz__' -exec \
    sh -c 'mv "${0}" "${0%.gz__}.gz" 2>/dev/null && gunzip "${0%.gz__}.gz"' {} \;

find "${OUTDIR}" -type f -name '*.deb' -exec \
    sh -c 'mv "{}" "${0}" 2>/dev/null' "${OUTDIR}" \;

find "${OUTDIR}" -type d -empty -delete

echo "Artifacts written to ${OUTDIR}"

popd
