#!/bin/bash
#
# Generates the PNG badges used in the Mingling README:
#   1. Stars   — GitHub star count of mingling-rs/mingling
#   2. License — fixed "MIT OR Apache-2.0"
#   3. Version — latest version of `mingling` on crates.io
#   4. Build   — status of the latest CI run (ci.yml)
#
# Requirements: curl + jq. Network access is needed: the badge font is
# downloaded from a CDN on first use (cached in .cache/), and the badge
# values come from the GitHub / crates.io APIs.

set -euo pipefail

cd "$(dirname "$0")"

REPO="mingling-rs/mingling"
CRATE="mingling"
WORKFLOW="ci.yml"
USER_AGENT="mingling-badges-gen (https://github.com/mingling-rs/mingling-badges)"

# fetch <name> <url> <jq-expr> — prints the extracted value or exits 1.
fetch() {
    local name="$1" url="$2" expr="$3"
    local value
    value=$(curl -fsS --max-time 30 -H "User-Agent: $USER_AGENT" "$url" | jq -r "$expr") || {
        echo "error: failed to fetch $name (url: $url)" >&2
        exit 1
    }
    printf '%s' "$value"
}

echo "Fetching badge data..."
stars=$(fetch "stars" "https://api.github.com/repos/$REPO" '.stargazers_count')
version=$(fetch "crates.io version" "https://crates.io/api/v1/crates/$CRATE" '.crate.max_version')

ci_run=$(fetch "CI status" \
    "https://api.github.com/repos/$REPO/actions/workflows/$WORKFLOW/runs?per_page=1" \
    '.workflow_runs[0]')
ci_status=$(printf '%s' "$ci_run" | jq -r '.status')
ci_conclusion=$(printf '%s' "$ci_run" | jq -r '.conclusion // "null"')
case "$ci_status:$ci_conclusion" in
    completed:success) build="PASSING" ;;
    completed:*)       build="FAIL" ;;
    *)                 build="RUNNING" ;;
esac

echo "  stars=$stars  version=$version  build=$build"

# `cargo badge` refuses to overwrite existing files.
rm -f badge-stars.png badge-license.png badge-version.png badge-build.png

echo "Generating badges..."
cargo badge --key="Stars" --value="$stars"
cargo badge --key="License" --value="MIT OR Apache-2.0"
cargo badge --key="Version" --value="$version"
cargo badge --key="Build" --value="$build"

# Move the finished badges into deploy/.
mkdir -p deploy
mv *.png deploy/

echo "Done: deploy/badge-stars.png deploy/badge-license.png deploy/badge-version.png deploy/badge-build.png"
