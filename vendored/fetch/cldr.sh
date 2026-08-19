#!/usr/bin/env bash
# Unicode CLDR, via the official JSON distribution — Unicode-3.0, which the FSF lists as
# GPL-compatible.
#
# CLDR is the only source that scales past a handful of languages without linear human effort:
# month and era names, per-locale number separators and currency placement, and unit display names,
# all versioned and authoritative. We take the `modern` variant — the locales in active use — and
# only the three files per locale we generate from.
#
# CLDR describes formatting. Turning a format pattern into a parser is our work, and an inverted
# pattern is always laxer than the formatter that produced it.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

target="${REFERENCE}/cldr"
replace_dir "$target"

take() {
    local package="$1" filename="$2"
    local src dest
    src="$(npm_tarball "$package")"
    dest="${target}/${package}"
    mkdir -p "$dest"
    ( cd "${src}/main" && find . -name "$filename" -exec cp --parents {} "${dest}/" \; )
    note "${package}: $(find "$dest" -name "$filename" | wc -l) locales"
}

take cldr-dates-modern ca-gregorian.json
take cldr-numbers-modern numbers.json
take cldr-units-modern units.json

# Territory names are runtime data rather than reference material: the explainer resolves a
# country-code top-level domain to the country it belongs to, and a card that says "GB" where it
# could say "United Kingdom" is a card nobody needed. Flattened to one map, because the release
# image should not carry CLDR's bundle structure for the sake of 300 strings.
names="$(npm_tarball cldr-localenames-modern)"
mkdir -p "${DATA}/cldr"
jq '.main.en.localeDisplayNames.territories
    | with_entries(select(.key | test("^[A-Z]{2}$")))' \
    "${names}/main/en/territories.json" > "${DATA}/cldr/territories-en.json"
stamp "${DATA}/cldr" "https://github.com/unicode-org/cldr-json (npm cldr-localenames-modern)" \
    "$(jq -r .version "${names}/package.json")"
note "territories: $(jq 'length' "${DATA}/cldr/territories-en.json") names"

# Supplemental data is locale-independent: currency minor units, week rules, likely subtags.
core="$(npm_tarball cldr-core)"
mkdir -p "${target}/cldr-core"
cp "${core}/supplemental/currencyData.json" "${target}/cldr-core/"
cp "${core}/supplemental/weekData.json" "${target}/cldr-core/"
cp "${core}/supplemental/likelySubtags.json" "${target}/cldr-core/"
cp "${core}/LICENSE" "${LICENSES}/cldr.LICENSE"

stamp "$target" "https://github.com/unicode-org/cldr-json (npm cldr-*-modern, cldr-core)" \
    "$(jq -r .version "${core}/package.json")"
