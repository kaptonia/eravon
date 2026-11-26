# galeed-core

The chain-facing references of the judge's ledger.

- `contracts/`: the GaleedRegistry, pinned for a reproducible codeHash
  (CODEHASH.md), with its forge suite: `publish(bytes)` for every document
  of the family, `setStatus(bool)`, and `anchorCarried(bytes32,bytes)` and
  `anchorCarriedMany(bytes32[],bytes[])`, which emit the one anchor form of
  galeed/1 section 9.1 and refuse bytes that do not hash to their word. The
  law reads the registry by the log shapes alone; the contract is the
  convenience that emits them.
  Beside it, the succession book `GaleedSuccessionBook` (`galeed.roster/1`
  section 4: `declare(address)`, `accede(address)`, the views `current`,
  `root`, `pending`, `entered`, and the `Declared` and `Acceded` logs) and
  the standard router `GaleedRouter` (`galeed.venue/1` section 4:
  `route(RouteView)` over a rule table fixed at construction, `rules()`),
  each with its forge suite and its codeHash in CODEHASH.md.
- `fixtures/`: seventy-six recordings with expected outputs, the chain-facing
  corpus: a scanner replays each recording and must reproduce the expected
  fragment byte for byte, or fail where the fixture records a failing scan.
  Each fragment is the output of the Python scanner in
  `../galeed-conformance/scan-py/`, checked against the facts each scenario
  asserts before the fixture is written; an independent scanner must
  reproduce every fragment.
