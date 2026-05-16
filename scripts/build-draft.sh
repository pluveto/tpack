#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GEM_BIN="$HOME/.local/share/gem/ruby/3.4.0/bin"
LOCAL_BIN="$HOME/.local/bin"
PATH="$GEM_BIN:$LOCAL_BIN:$PATH"
export PATH

cd "$ROOT"

kramdown-rfc2629 drafts/draft-zhang-tpack-format-00.md > drafts/draft-zhang-tpack-format-00.xml
xml2rfc --v3 --text --html --cache .tmp/xml2rfc-cache drafts/draft-zhang-tpack-format-00.xml
