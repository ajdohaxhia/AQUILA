#!/usr/bin/env bash
# Copyright (C) 2026 Michael Lane
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

# Waits for the CI run of a commit on GitHub and, only if it ends green, starts the AQUILA release workflow
# (.github/workflows/aquila-release.yml). Needs the GitHub CLI (`gh`), logged in with access to the repository.
#
# Usage: packaging/release-when-green.sh [commit] [version]
#   commit   the commit whose CI run decides (default: the local HEAD, which must be pushed)
#   version  e.g. 2026.11.17.3 (default: the tag in packaging/github-release.json, as the workflow reads it)
#
# Environment: REPO (default ajdohaxhia/AQUILA), REF (branch the release builds, default michael-lane),
# POLL (seconds between checks, default 60), WAIT_START (seconds to wait for the run to appear, default 600).
set -euo pipefail

# Keep the Mac awake while waiting (a sleeping Mac cannot see the CI finish).
if [ "$(uname)" = Darwin ] && [ -z "${CAFFEINATED:-}" ] && command -v caffeinate >/dev/null; then
  CAFFEINATED=1 exec caffeinate -i "$0" "$@"
fi

REPO="${REPO:-ajdohaxhia/AQUILA}"
REF="${REF:-michael-lane}"
POLL="${POLL:-60}"
WAIT_START="${WAIT_START:-600}"
SHA="$(git rev-parse "${1:-HEAD}")"
VERSION="${2:-}"

say() { printf '[%s] %s\n' "$(date '+%H:%M:%S')" "$*"; }

command -v gh >/dev/null || { echo "gh (GitHub CLI) is required" >&2; exit 2; }

# The CI run for this commit (the newest, if it ran more than once).
run_id=""
waited=0
until [ -n "$run_id" ]; do
  run_id="$(gh run list -R "$REPO" --workflow ci.yml --commit "$SHA" -L 1 --json databaseId -q '.[0].databaseId' 2>/dev/null || true)"
  if [ -z "$run_id" ]; then
    if [ "$waited" -ge "$WAIT_START" ]; then
      echo "No CI run found for ${SHA:0:8} after ${WAIT_START}s. Was the commit pushed?" >&2
      exit 3
    fi
    sleep 15; waited=$((waited + 15))
  fi
done
say "CI run $run_id for ${SHA:0:8}: https://github.com/$REPO/actions/runs/$run_id"

# Wait for it to finish.
while :; do
  # A network hiccup must not end the wait: try again on the next poll.
  if ! state="$(gh run view "$run_id" -R "$REPO" --json status,conclusion -q '"\(.status) \(.conclusion)"' 2>/dev/null)"; then
    say "GitHub not reachable, retrying..."
    sleep "$POLL"
    continue
  fi
  read -r status conclusion <<<"$state"
  [ "$status" = "completed" ] && break
  say "CI is $status..."
  sleep "$POLL"
done

if [ "$conclusion" != "success" ]; then
  say "CI ended with '$conclusion': no release. Jobs:"
  gh run view "$run_id" -R "$REPO" --json jobs -q '.jobs[] | "  \(.name): \(.conclusion)"'
  exit 1
fi

say "CI is green. Starting the AQUILA release from $REF${VERSION:+ (version $VERSION)}."
args=(workflow run aquila-release.yml -R "$REPO" --ref "$REF")
[ -n "$VERSION" ] && args+=(-f "version=$VERSION")
gh "${args[@]}"
sleep 10
gh run list -R "$REPO" --workflow aquila-release.yml -L 1
