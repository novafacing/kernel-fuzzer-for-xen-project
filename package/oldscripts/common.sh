#!/bin/bash

#################################
# LOGGING
#################################

# This logfile is output on failure, all relevant output
# should be written or redirected here
LOGDIR="/log"
if [ ! -d "${LOGDIR}" ]; then
    mkdir -p "${LOGDIR}"
fi
LOGFILE="${LOGDIR}/logfile.txt"

# Set to true to log to stderr as well as the logfile
QUIET=false

# Echo to stderr
echoerr() { echo "$@" 1>&2; }

log_output() {
  if [[ "${QUIET}" != true ]]; then
    echoerr "|$1|$(date '+%Y-%m-%d %H:%M:%S')| $2" 
  fi

  echo "|$1|$(date '+%Y-%m-%d %H:%M:%S')| $2" >> "${LOGFILE}"
}

log_debug() {
    log_output "#" "$*"
}

log_info() {
    log_output "*" "$*"
}

log_warn() {
    log_output "~" "$*"
}

log_error() {
    log_output "!" "$*"
}

fail() {
    log_error "Fatal error occurred building KF/x! Dumping log."
    cat /tmp/logfile.txt
}

#################################
# Utility
#################################