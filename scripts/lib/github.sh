#!/usr/bin/env bash
# Shared GitHub release helpers for install scripts.
set -euo pipefail

github_api_token() {
  for key in GITHUB_TOKEN GH_TOKEN GITHUB_PERSONAL_ACCESS_TOKEN; do
    if [[ -n "${!key:-}" ]]; then
      echo "${!key}"
      return 0
    fi
  done
  return 1
}

github_curl() {
  local url="$1"
  local -a args=(-fsSL -H "Accept: application/vnd.github+json" -H "User-Agent: autonomic-ai")
  local token
  if token="$(github_api_token 2>/dev/null)"; then
    args+=(-H "Authorization: Bearer ${token}")
  fi
  args+=("$url")
  curl "${args[@]}"
}

# Redirect-first tag resolution; API fallback with optional token.
github_latest_tag() {
  local repo="$1"
  local binary="$2"
  local target="$3"
  local asset="${binary}-${target}"
  local probe_url="https://github.com/${repo}/releases/latest/download/${asset}"

  local location
  if location="$(curl -fsS -o /dev/null -D - "$probe_url" | awk 'tolower($1)=="location:"{print $2}' | tr -d '\r' | tail -1)"; then
    if [[ -n "$location" ]]; then
      local tag
      tag="$(echo "$location" | sed -n 's|.*/releases/download/\([^/]*\)/.*|\1|p')"
      if [[ -n "$tag" ]]; then
        echo "$tag"
        return 0
      fi
    fi
  fi

  github_curl "https://api.github.com/repos/${repo}/releases/latest" | awk -F'"' '/"tag_name"/{print $4; exit}'
}
