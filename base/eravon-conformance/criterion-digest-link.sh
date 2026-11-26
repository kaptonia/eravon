#!/bin/sh
# The release digest of the eravon.link/1 criterion (docs/eravon-link-v1.md
# s11.5): sha256 over the lines "<sha256 of file>  <name>\n" for the source
# files of link-py in this fixed order, followed by the line
# "<eravon.terms/1 core digest>  eravon.terms/1\n" as criterion-digest-terms.sh prints it. Run from anywhere; python3 only.
# Two guards, tooling: the upstream digest this script embeds is the one the
# law's pinning section quotes (where it quotes one), and the digest this
# script prints is the one the law's naming section names (where it names
# one); a wrong tree prints nothing.
set -e
DIR="$(cd "$(dirname "$0")/link-py" && pwd)"
LAW="$(dirname "$0")/../docs/eravon-link-v1.md"
if [ ! -f "$LAW" ]; then
  echo "law text $LAW is not beside this script's tree" >&2
  exit 1
fi
TERMS=$(sh "$(dirname "$0")/criterion-digest-terms.sh")
PINNED=$(sed -n '/^\*\*11.4\*\*/,/^\*\*11.5 /p' "$LAW" | grep -oE '0x[0-9a-f]{64}' | head -1)
if [ -n "$PINNED" ] && [ "$PINNED" != "$TERMS" ]; then
  echo "eravon.terms/1 core $TERMS is not the one $LAW s11.4 names ($PINNED)" >&2
  exit 1
fi
LISTING=""
for f in erl.py erllaw.py; do
  H=$(python3 -c "import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$DIR/$f")
  LISTING="$LISTING$H  $f
"
done
LISTING="$LISTING$TERMS  eravon.terms/1
"
OUT=$(printf '%s' "$LISTING" | python3 -c "import hashlib,sys; print('0x'+hashlib.sha256(sys.stdin.buffer.read()).hexdigest())")
NAMED=$(sed -n '/^\*\*11.5 /,/^---/p' "$LAW" | grep -oE '0x[0-9a-f]{64}' | head -1)
if [ -n "$NAMED" ] && [ "$NAMED" != "$OUT" ]; then
  echo "computed $OUT is not the digest $LAW s11.5 names ($NAMED)" >&2
  exit 1
fi
echo "$OUT"
