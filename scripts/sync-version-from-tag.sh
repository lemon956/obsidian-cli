#!/usr/bin/env bash
set -euo pipefail

tag="${1:-}"
if [[ -z "$tag" ]]; then
  tag="$(git describe --tags --exact-match HEAD)"
fi

if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  echo "expected a semver tag like v0.1.3, got: $tag" >&2
  exit 1
fi

version="${tag#v}"

perl -0pi -e 's/(\[package\]\s*\n(?:[^\[]*\n)*?version\s*=\s*")[^"]+(")/${1}'"$version"'${2}/s' Cargo.toml

perl -0pi -e 's/(name\s*=\s*"obsidian-cli"\s*\nversion\s*=\s*")[^"]+(")/${1}'"$version"'${2}/s' Cargo.lock

if [[ -f README.md ]]; then
  perl -0pi -e 's/git tag v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)? && git push origin v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?/git tag '"$tag"' && git push origin '"$tag"'/g' README.md
fi

echo "Synced project version to $version from $tag"
