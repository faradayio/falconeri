#!/usr/bin/env bash

# Standard paranoia.
set -euo pipefail

for F in /pfs/texts/*.txt; do
    outfile="/pfs/out/$(basename "$F")"
    cat "$F" |
        tr '[:upper:]' '[:lower:]' |
        tr -c '[:alpha:]' ' ' |
        awk '{ for (i=1; i <= NF; i++) { print $i } }' |
        sort |
        uniq -c |
        sort -nr > "$outfile"
done
