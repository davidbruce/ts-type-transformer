#!/bin/sh
# Installs the pinned libraries into target/libraries, as the lockfile has them.
set -e
root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$root/target/libraries"
cp "$root/tests/libraries/package.json" "$root/tests/libraries/package-lock.json" "$root/target/libraries/"
cd "$root/target/libraries"
npm ci --ignore-scripts --no-audit --no-fund
