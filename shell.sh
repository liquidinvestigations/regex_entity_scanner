#!/usr/bin/env bash
#
# Interactive shell in a throwaway dev container on the working tree. Any arguments are run as a
# command instead of starting a shell:  ./shell.sh cargo test

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

ensure_dev_image

if [[ $# -eq 0 ]]; then
    set -- bash
fi

exec docker run --rm --interactive --tty \
    "${DEV_RUN_ARGS[@]}" \
    "$DEV_IMAGE" \
    "$@"
