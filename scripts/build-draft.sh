#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GEM_BIN="$HOME/.local/share/gem/ruby/3.4.0/bin"
LOCAL_BIN="$HOME/.local/bin"
PATH="$GEM_BIN:$LOCAL_BIN:$PATH"
export PATH

cd "$ROOT"

if [ "$#" -gt 1 ]; then
  echo "usage: $0 [draft-source.md]" >&2
  exit 1
fi

if [ "$#" -eq 1 ]; then
  DRAFT_SOURCE=$1
else
  set -- drafts/draft-zhang-tpack-format-*.md
  if [ ! -f "$1" ]; then
    echo "no draft source found under drafts/" >&2
    exit 1
  fi
  DRAFT_SOURCE=$1
  for candidate in "$@"; do
    DRAFT_SOURCE=$candidate
  done
fi

DRAFT_BASE=${DRAFT_SOURCE%.md}

kramdown-rfc2629 "$DRAFT_SOURCE" > "$DRAFT_BASE.xml"
xml2rfc --v3 --text --html --cache .tmp/xml2rfc-cache "$DRAFT_BASE.xml"
