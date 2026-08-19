#!/usr/bin/env bash
# logstash-patterns-core — Apache-2.0.
# Battle-tested building blocks for the machine formats that fill logs and mail headers: syslog and
# common-log timestamps, IPs, URIs, paths, UUIDs, MACs. Machine formats are the highest-precision
# date material in any corpus, and these are the shapes worth special-casing.

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"

src="$(clone https://github.com/logstash-plugins/logstash-patterns-core.git logstash-patterns-core)"
target="${REFERENCE}/grok"
replace_dir "$target"
cp -r "${src}/patterns" "${target}/patterns"
cp "${src}/LICENSE" "${LICENSES}/logstash-patterns-core.LICENSE"
stamp "$target" "https://github.com/logstash-plugins/logstash-patterns-core" "$(git_head "$src")"
