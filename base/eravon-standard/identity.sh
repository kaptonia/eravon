#!/bin/sh
# Recomputes the standard's identity from the pinned compile (standard s13)
# and compares it with CODEHASH.md. Run inside eravon/standard.
set -e
cd "$(dirname "$0")"
forge build --silent
out=$(forge script script/Identity.s.sol 2>/dev/null | grep -E "^  (logicInitcodeHash|logic|proxyCodeHash|panelInitcodeHash|panel|panelCodeHash|verifierInitcodeHash|verifier|verifierCodeHash|boardInitcodeHash|board|boardCodeHash) " | sed 's/^  //')
if [ -z "$out" ]; then
  echo "identity: the script printed no identity lines (run forge clean; forge build first)" >&2
  exit 1
fi
echo "$out"
cur=$(awk '/^\| 1 \|/{t=1} t && !/^\| /{exit} {print}' CODEHASH.md)
for line in $(echo "$out" | tr ' ' '='); do
  key=${line%%=*}; val=${line#*=}
  printf '%s\n' "$cur" | grep -q "| \`$key\` | \`$val\` |" || { echo "CODEHASH.md's current table disagrees on $key" >&2; exit 1; }
done
echo "CODEHASH.md agrees"
