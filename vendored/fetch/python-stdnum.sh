#!/usr/bin/env bash
# python-stdnum — LGPL-2.1+, which upgrades cleanly into an AGPLv3 work.
# 200+ number formats with their real check-digit algorithms: IBAN, BIC, EU VAT, company
# registration numbers across some sixty jurisdictions, LEI, ISIN, national identity numbers, IMO,
# MMSI, ISO 6346 containers, IMEI. Each module is a compact format-plus-validator pair, which makes
# this the specification we port from — a check digit turns "matched something plausible" into
# "verified", and that is the difference between a join key and index noise.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/arthurdejong/python-stdnum.git python-stdnum)"
target="${REFERENCE}/python-stdnum"
replace_dir "$target"
cp -r "${src}/stdnum" "${target}/stdnum"
# The bundled data files are large downloads refreshed at install time upstream, not part of the
# specification we port.
find "${target}/stdnum" -name '*.dat' -delete
cp "${src}/COPYING" "${LICENSES}/python-stdnum.LICENSE"
stamp "$target" "https://github.com/arthurdejong/python-stdnum" "$(git_head "$src")"
note "$(find "${target}/stdnum" -name '*.py' | wc -l) format modules"
