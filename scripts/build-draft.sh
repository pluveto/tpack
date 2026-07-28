#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
GEM_BIN="$HOME/.local/share/gem/ruby/3.4.0/bin"
LOCAL_BIN="$HOME/.local/bin"
PATH="$GEM_BIN:$LOCAL_BIN:$PATH"
export PATH

# Optional override for the Ruby image used when kramdown-rfc is not installed
# locally (scheme B: Docker-only draft toolchain for the markdown→XML step).
KRAMDOWN_DOCKER_IMAGE=${KRAMDOWN_DOCKER_IMAGE:-ruby:3.3-slim}
KRAMDOWN_GEM_CACHE_VOLUME=${KRAMDOWN_GEM_CACHE_VOLUME:-tpack-kramdown-gem-cache}

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

case "$DRAFT_SOURCE" in
  /*) ;;
  *) DRAFT_SOURCE="$ROOT/$DRAFT_SOURCE" ;;
esac

if [ ! -f "$DRAFT_SOURCE" ]; then
  echo "draft source not found: $DRAFT_SOURCE" >&2
  exit 1
fi

# Keep paths relative to ROOT so Docker bind mounts and local tools agree.
DRAFT_REL=${DRAFT_SOURCE#"$ROOT"/}
if [ "$DRAFT_REL" = "$DRAFT_SOURCE" ]; then
  echo "draft source must live under $ROOT" >&2
  exit 1
fi

DRAFT_BASE=${DRAFT_REL%.md}

run_kramdown_rfc() {
  if command -v kramdown-rfc2629 >/dev/null 2>&1; then
    echo "build-draft: using local kramdown-rfc2629" >&2
    kramdown-rfc2629 "$DRAFT_REL"
    return
  fi
  if command -v kramdown-rfc >/dev/null 2>&1; then
    echo "build-draft: using local kramdown-rfc" >&2
    kramdown-rfc "$DRAFT_REL"
    return
  fi
  if ! command -v docker >/dev/null 2>&1; then
    echo "build-draft: kramdown-rfc not installed, and docker is unavailable." >&2
    echo "Install Ruby gem 'kramdown-rfc', or install Docker (scheme B)." >&2
    exit 1
  fi

  echo "build-draft: using Docker image $KRAMDOWN_DOCKER_IMAGE for kramdown-rfc" >&2
  # Gem cache volume avoids reinstalling kramdown-rfc on every run.
  # Keep gem install noise off stdout so only RFCXML is redirected to .xml.
  docker run --rm \
    -v "$ROOT:/work" \
    -v "$KRAMDOWN_GEM_CACHE_VOLUME:/usr/local/bundle" \
    -w /work \
    "$KRAMDOWN_DOCKER_IMAGE" \
    sh -c '
      set -eu
      if ! gem list -i kramdown-rfc >/dev/null 2>&1; then
        echo "build-draft(docker): installing kramdown-rfc into cache volume..." >&2
        gem install --no-document kramdown-rfc >/dev/null
      fi
      if command -v kramdown-rfc2629 >/dev/null 2>&1; then
        exec kramdown-rfc2629 "$1"
      fi
      exec kramdown-rfc "$1"
    ' sh "$DRAFT_REL"
}

run_kramdown_rfc > "$DRAFT_BASE.xml"

if ! command -v xml2rfc >/dev/null 2>&1; then
  echo "build-draft: xml2rfc not found in PATH (expected under ~/.local/bin)." >&2
  exit 1
fi

mkdir -p .tmp/xml2rfc-cache
xml2rfc --v3 --text --html --cache .tmp/xml2rfc-cache "$DRAFT_BASE.xml"

echo "build-draft: wrote $DRAFT_BASE.xml $DRAFT_BASE.txt $DRAFT_BASE.html" >&2
