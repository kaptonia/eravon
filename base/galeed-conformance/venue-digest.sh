#!/bin/sh
# The identity digest of galeed.venue/1 (docs/galeed-venue-v1.md §10): sha256
# over the bytes of galeed-conformance/venue-replay/src/IGaleedVenue.sol, the
# interface verbatim. Run from anywhere; python3 only.
set -e
F="$(cd "$(dirname "$0")/venue-replay/src" && pwd)/IGaleedVenue.sol"
python3 -c "import hashlib,sys; print('0x'+hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())" "$F"
