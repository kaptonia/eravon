#!/bin/sh
# The release digest of the eravon.terms/1 criterion (docs/eravon-terms-v1.md
# s10.5): sha256 over the lines "<sha256 of file>  <name>\n" for the source
# files of terms-py in this fixed order, the files of s10.4 among them. Run
# from anywhere; python3 only.
# A guard, tooling: the digest this script prints is the one the law's naming
# section names (where it names one); a wrong tree prints nothing.
set -e
DIR="$(cd "$(dirname "$0")/terms-py" && pwd)"
LAW="$(dirname "$0")/../docs/eravon-terms-v1.md"
if [ ! -f "$LAW" ]; then
  echo "law text $LAW is not beside this script's tree" >&2
  exit 1
fi
LISTING=""
for f in ert.py ertlaw.py ertcrypto.py ertcanon.py; do
  H=$(python3 -c "import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$DIR/$f")
  LISTING="$LISTING$H  $f
"
done
OUT=$(printf '%s' "$LISTING" | python3 -c "import hashlib,sys; print('0x'+hashlib.sha256(sys.stdin.buffer.read()).hexdigest())")
NAMED=$(sed -n '/^\*\*10.5 /,/^---/p' "$LAW" | grep -oE '0x[0-9a-f]{64}' | head -1)
if [ -n "$NAMED" ] && [ "$NAMED" != "$OUT" ]; then
  echo "computed $OUT is not the digest $LAW s10.5 names ($NAMED)" >&2
  exit 1
fi
echo "$OUT"
