#!/bin/bash

usage() {
    echo "usage: package.sh <IMAGE> [OUTDIR]"
    echo "  IMAGE:  The name of the image to build on, or 'all'."
    echo "    Options: 'all', 'jammy', 'focal', 'buster', 'bullseye', 'bionic'"
    echo "  OUTDIR: Optional path to output directory for .deb files."
    echo "    Default: creates a new temp directory."
    exit 1
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

echo "Running with ${WORKFLOW_FILE}"

act -W "${WORKFLOW_FILE}" --artifact-server-path="${OUTDIR}"

find "${OUTDIR}" -type f -name '*.gz__' -exec \
    sh -c 'mv "${0}" "${0%.gz__}.gz" && gunzip "${0%.gz__}.gz"' {} \;

find "${OUTDIR}" -type f -name '*.deb' -exec \
    sh -c 'mv "{}" "${0}"' "${OUTDIR}" \;

find "${OUTDIR}" -type d -empty -delete

echo "Artifacts written to ${OUTDIR}"

popd
