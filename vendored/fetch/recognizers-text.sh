#!/usr/bin/env bash
# Microsoft Recognizers-Text — MIT.
# The language-neutral YAML under Patterns/ is the whole reason to vendor this: curated, already
# false-positive-tuned pattern and lexicon files for dates, currency, units, phones, email and more,
# in fifteen languages, kept separate from the C#/JS/Python implementations.
#
# The patterns are .NET flavour and lean on lookaround, which no linear-time engine accepts. They
# are input to our rule authoring, not something we compile as-is.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/microsoft/Recognizers-Text.git recognizers-text)"
target="${REFERENCE}/recognizers-text"
replace_dir "$target"
cp -r "${src}/Patterns" "${target}/Patterns"
cp "${src}/LICENSE" "${LICENSES}/recognizers-text.LICENSE"
stamp "$target" "https://github.com/microsoft/Recognizers-Text" "$(git_head "$src")"
note "$(find "${target}/Patterns" -name '*.yaml' | wc -l) pattern files"
