#!/usr/bin/env bash
# Microsoft Presidio — MIT.
# Not our engine — it is Python, it has neither money nor units, and it emits spans for redaction
# rather than canonical values. Its predefined recognisers are still the best available checklist of
# PII entity types and per-country identifier formats, each declaring whether it validates by
# pattern, by checksum or by context.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/microsoft/presidio.git presidio)"
target="${REFERENCE}/presidio"
replace_dir "$target"
cp -r "${src}/presidio-analyzer/presidio_analyzer/predefined_recognizers" "${target}/"
cp "${src}/LICENSE" "${LICENSES}/presidio.LICENSE"
stamp "$target" "https://github.com/microsoft/presidio" "$(git_head "$src")"
note "$(find "${target}" -name '*.py' | wc -l) recogniser modules"
