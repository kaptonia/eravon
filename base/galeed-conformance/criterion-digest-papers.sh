#!/bin/sh
# The release digest of the galeed.papers/1 criterion (docs/galeed-papers-v1.md
# §13.5): sha256 over the lines "<sha256 of file>  <name>\n" for the three
# source files of papers-py in this fixed order, followed by the line
# "<galeed/1 core digest>  galeed/1\n" as criterion-digest.sh prints it.
# Run from anywhere; python3 only. The law line this script reads is one
# holding only the backticked galeed/1 core digest, as the naming writes it.
set -e
DIR="$(cd "$(dirname "$0")/papers-py" && pwd)"
CORE=$(sh "$(dirname "$0")/criterion-digest.sh")
# the embedded core must be the one the law's s13.4 names; any other core is
# another core, and this script prints nothing for it
LAW="$(dirname "$0")/../docs/galeed-papers-v1.md"
if [ ! -f "$LAW" ]; then
  echo "law text $LAW is not beside this script's tree" >&2
  exit 1
fi
if ! grep -q "^\`$CORE\`\$" "$LAW"; then
  echo "core $CORE is not the galeed/1 core named in $LAW s13.4" >&2
  exit 1
fi
LISTING=""
for f in gpp.py gppdoc.py gppdoor.py; do
  H=$(python3 -c "import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$DIR/$f")
  LISTING="$LISTING$H  $f
"
done
LISTING="$LISTING$CORE  galeed/1
"
printf '%s' "$LISTING" | python3 -c "import hashlib,sys; print('0x'+hashlib.sha256(sys.stdin.buffer.read()).hexdigest())"
