#!/bin/bash

OUTDIR="${1}"

if [ -z "${OUTDIR}" ]; then
    OUTDIR=$(mktemp -p /tmp -d kfx-artifacts-XXXXXX)
fi

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "${SCRIPT_DIR}/.." > /dev/null

git submodule update --init --recursive

act -W "${SCRIPT_DIR}/package.yml" --artifact-server-path="${OUTDIR}"

find "${OUTDIR}" -type f -name '*.gz__' -exec \
    sh -c 'mv "${0}" "${0%.gz__}.gz" && gunzip "${0%.gz__}.gz"' {} \;

find "${OUTDIR}" -type f -name '*.deb' -exec \
    sh -c 'mv "{}" "${0}"' "${OUTDIR}" \;

find "${OUTDIR}" -type d -empty -delete

echo "Artifacts written to ${OUTDIR}"

popd
