#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
PI_AGENT_DIR="${PI_CODING_AGENT_DIR:-${HOME}/.pi/agent}"
CARGO_ROOT="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-${HOME}/.cargo}}"
EXTENSION_SOURCE="${ROOT_DIR}/extensions/issues.ts"
EXTENSION_DEST="${PI_AGENT_DIR}/extensions/issues.ts"

if ! command -v cargo >/dev/null 2>&1; then
  printf 'error: cargo is required to install the issues CLI.\n' >&2
  exit 1
fi

if [[ ! -f "${EXTENSION_SOURCE}" ]]; then
  printf 'error: extension source not found: %s\n' "${EXTENSION_SOURCE}" >&2
  exit 1
fi

printf 'Installing issues CLI from %s\n' "${ROOT_DIR}"
cargo install \
  --path "${ROOT_DIR}" \
  --locked \
  --force \
  --root "${CARGO_ROOT}"

mkdir -p "$(dirname -- "${EXTENSION_DEST}")"
install -m 0644 "${EXTENSION_SOURCE}" "${EXTENSION_DEST}"

printf 'Installed issues CLI: %s\n' "${CARGO_ROOT}/bin/issues"
printf 'Installed Pi extension: %s\n' "${EXTENSION_DEST}"
printf 'Run /reload in Pi (or restart Pi) to load the extension.\n'

if [[ ":${PATH}:" != *":${CARGO_ROOT}/bin:"* ]]; then
  printf 'note: add %s/bin to PATH if the issues tool cannot find the CLI.\n' "${CARGO_ROOT}"
fi
