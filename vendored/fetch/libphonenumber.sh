#!/usr/bin/env bash
# libphonenumber (Google) — Apache-2.0.
# The metadata itself reaches us through the `phonenumber` crate, which tracks upstream and keeps us
# on an update path; regulators reassign number ranges continuously, so a frozen copy would rot.
#
# What we vendor is the part the crate does not provide: `PhoneNumberMatcher`, the reference
# implementation of finding numbers in free text. It is the two-stage design in its original form —
# a loose candidate pattern, then parse, validate, and a set of post-checks on the surrounding
# characters — and porting its heuristics is the highest-value custom work in the phone rule.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/google/libphonenumber.git libphonenumber)"
target="${REFERENCE}/libphonenumber"
replace_dir "$target"

java="${src}/java/libphonenumber/src/com/google/i18n/phonenumbers"
for file in PhoneNumberMatcher.java PhoneNumberUtil.java; do
    cp "${java}/${file}" "${target}/"
done
cp "${src}/LICENSE" "${LICENSES}/libphonenumber.LICENSE"
stamp "$target" "https://github.com/google/libphonenumber" "$(git_head "$src")"
