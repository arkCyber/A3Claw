#!/usr/bin/env bash
#
# Setup long-term Git proxy for GitHub (and optional global).
# Usage:
#   ./scripts/setup_git_proxy.sh              # use default http://127.0.0.1:7890
#   PROXY_URL=socks5://127.0.0.1:1080 ./scripts/setup_git_proxy.sh
#   ./scripts/setup_git_proxy.sh --unset     # remove proxy
#
set -e

GITHUB_PROXY="${PROXY_URL:-${https_proxy:-${HTTPS_PROXY:-http://127.0.0.1:7890}}}"

if [ "$1" = "--unset" ]; then
  git config --global --unset-all http.https://github.com.proxy 2>/dev/null || true
  git config --global --unset-all https.https://github.com.proxy 2>/dev/null || true
  echo "Git proxy for GitHub has been removed."
  exit 0
fi

git config --global http.https://github.com.proxy "$GITHUB_PROXY"
git config --global https.https://github.com.proxy "$GITHUB_PROXY"
echo "Git proxy for GitHub set to: $GITHUB_PROXY"
echo "Verify: git config --global --get http.https://github.com.proxy"
