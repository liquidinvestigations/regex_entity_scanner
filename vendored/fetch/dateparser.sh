#!/usr/bin/env bash
# dateparser (Zyte) — BSD-3.
# Its CLDR-derived translation tables cover 200+ locales with month and weekday names, separators
# and skip-words, plus a hand-curated supplementary layer for what CLDR misses. It is data rather
# than code, so it ports to Rust as a transform rather than a rewrite.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/scrapinghub/dateparser.git dateparser)"
target="${REFERENCE}/dateparser"
replace_dir "$target"
cp -r "${src}/dateparser_data" "${target}/dateparser_data"
cp "${src}/LICENSE" "${LICENSES}/dateparser.LICENSE"
stamp "$target" "https://github.com/scrapinghub/dateparser" "$(git_head "$src")"
note "$(find "${target}" -name '*.yaml' -o -name '*.json' | wc -l) locale files"
