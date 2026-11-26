# galeed-conformance

The criterion layer of `galeed/1` (docs/galeed-v1.md): what an implementation
is measured against, and the tools that measure it. Nothing here is an
implementation for use; the implementations below the law are born elsewhere
and read against this.

- `impl-py/`: the core of `galeed/1`, pure Python 3 standard library
  (`gl1.py`, `glcanon.py`, `glcrypto.py`, `glentry.py`, `glaudit.py`). Its
  release digest is computed by `criterion-digest.sh` and named by §12.5 of
  the law; from that naming the core binds and the law is its textbook. The
  law was revised in place on 2026-10-16 (the chain as the ledger: an audit
  of two inputs, one anchor form carrying the entry) and on 2026-10-23 (a
  key's retirement across chains, the `migration` entry and the
  continuation of §12.3) and on 2026-10-30 (the second of the handing off: a
  key has retired at a record of its retirement's own second that carries
  what it signed at a position after its retiring seq, the smallest position
  it handed off at in that second), and §12.5 names the digest of the
  revised core at its naming.
- `HARNESS.md`: the command-line contract every candidate implements
  (`gl1 check|canon|sign|audit`).
- `corpus/gen.py`: the corpus generator, written from the law and the harness
  alone, carrying its own signing path and its own predictions as a third
  opinion. `python3 gen.py --seed N` rebuilds `corpus/canon`, `corpus/sign`,
  `corpus/audit` and `manifest.json` deterministically. Seed 1 is committed.
- `compare.py`: runs a candidate named by `GL1_CANDIDATE` and the criterion
  over the corpus (`GL1_CORPUS` names another corpus directory) and counts
  every divergence and every disagreement with the generator's prediction,
  the misuse cases and the sign cases expecting exit 2 included, printing
  the first forty of the disagreements; without a candidate it checks the
  criterion against the predictions alone and says so. The manifest defines
  the corpus: every entry is a case, and the files it names must exist. A
  case passes only where its comparison ran to the end and found everything
  equal; an answer that does not parse, a predicted member the answer lacks,
  an audit answer of another shape than its predicted label's (`NO_LABEL` is
  exactly `{"ok":false,"reason":"NO_LABEL"}`), an entry lacking a prediction
  key of its category, an argument-vector case with a prediction other than
  null, a file the manifest names that is absent, a file of the corpus no
  entry names, an unreadable manifest and a corpus with no case are
  disagreements. The exit code is 1 where any divergence or disagreement was
  found, 0 otherwise; the divergence record goes to `GL1_DIVERGENCES` or to
  `divergences.json` in the working directory.
- `scan-py/`: an independent Python scanner for the chain-facing §9
  (`scan_replay.py`), replayed against the seventy-six recordings in
  `../galeed-core/fixtures/`; `scenarios.py` produces those recordings
  against anvil and `NOTES.md` records every choice the text left open.
- `bcov.py`: branch-arm witnessing of the criterion and the scanner over
  their corpora (a measure, never a criterion); `BOUNDARY-AUDIT.md` reads
  the result under seeds 1 to 6.

## The venue replay (`galeed.venue/1`, docs/galeed-venue-v1.md)

- `venue-replay/`: §8 of the venue interface as an abstract Foundry test
  (`src/VenueReplay.sol`) over the interface verbatim
  (`src/IGaleedVenue.sol`). A venue completes it by implementing the hooks
  (deploy, whether it draws, whether a seating waits for `accept`, its
  own creation and dispute paths, a party) in a concrete test contract
  and running `forge test` there; every assertion names the spec line it
  tests. The replay deploys the succession book of `galeed.roster/1` §4
  (`src/GaleedSuccessionBook.sol`, the core's contract byte for byte) before
  the venue, moves the judge's and the guild's keys on it, and engages
  panels with the routers of `src/ReplayRouters.sol`.
  `test/MinimalVenue.sol`, `test/MinimalDrawingVenue.sol` and
  `test/MinimalBookVenue.sol` (with `test/MinimalPanelBook.sol`) are the
  harness's own reference venues, one naming its judge in the terms, one
  drawing from panels it holds and one drawing from a panel book, which
  prove the replay runs on the three branches of §4 and are no venues for
  use. Tooling only: the
  interface binds by its text, and a change here is never a change to it.

## The papers layer (`galeed.papers/1`, docs/galeed-papers-v1.md)

- `papers-py/`: the criterion candidate of the paper law, pure Python 3
  over the `galeed/1` core above (`gpp.py`, `gppdoc.py`,
  `gppdoor.py`). Its release digest is computed by
  `criterion-digest-papers.sh` (sha256 over the three files' digest lines
  followed by the `galeed/1` core digest) and named by §13.5 of that law;
  from that naming the core binds and the law is its textbook.
- `HARNESS-PAPERS.md`: the command-line contract every candidate implements
  (`gpp note-check|pass-check|report-check|refusal-check|service-check|sign|pair|pair-refusal|qr-encode|qr-decode|service-of|serving|door`).
- `papers-corpus/gen.py`: the corpus generator with its own predictions;
  seed 1 is committed.
- `compare-papers.py`: runs a candidate named by `GPP_CANDIDATE` and the
  criterion over the papers corpus (a directory given as its argument names
  another); without a candidate it checks the criterion against the
  predictions alone and says so. The manifest defines the corpus: every
  corpus path a case names must carry a file, except the paths the case
  lists as `absent`, which must carry none. A case passes where the two
  sides agree and the criterion's answer equals the prediction byte for byte
  (exit 2 and no output where the prediction is null); a file present or
  absent against the manifest, a corpus with no case and an unreadable
  manifest are disagreements. It prints the first sixty disagreements and
  exits 1 where any divergence or disagreement was found, 0 otherwise.
- `bcov.py papers <out.json>`: branch-arm witnessing of the papers
  criterion over its corpus.

## The roster layer (`galeed.roster/1`, docs/galeed-roster-v1.md)

- `roster-py/`: the criterion candidate of the registry law, pure Python 3
  over the `galeed/1` core above (`gro.py`, `grolaw.py`), for the
  reader, `keccak256`, `sha256`, the basis well-formedness test and the
  canonical form of its answers. Its release digest is computed by
  `criterion-digest-roster.sh` (sha256 over the two files' digest lines
  followed by the `galeed/1` core digest) and named by §10.5 of that law.
- `HARNESS-ROSTER.md`: the command-line contract every candidate implements
  (`gro decode|publication|resolve-identity|current-at|resolve-guild|judge|guild-card|resolve-judge|covenant|quote|bench|candidates`).
- `roster-corpus/gen.py`: the corpus generator with hand-written
  predictions (660 cases); seed 1 is committed; the README carries the
  boundary list the law's §10.1 makes a precondition.
- `compare-roster.py`: runs a candidate named by `GRO_CANDIDATE` and the
  criterion over the roster corpus; without a candidate it checks the
  criterion against the predictions alone and says so. The manifest defines
  the corpus: every corpus path a case names must carry a file, except the
  paths the case lists as `absent`, which must carry none. A case passes
  where the two sides agree and the criterion's answer equals the prediction
  byte for byte (exit 2 and no output where the prediction is null); a file
  present or absent against the manifest, a corpus with no case and an
  unreadable manifest are disagreements. It prints the first sixty
  disagreements and exits 1 where any divergence or disagreement was found,
  0 otherwise.
- `bcov.py roster <out.json>`: branch-arm witnessing of the registry
  criterion over its corpus.

## The evidence layer (`courtevidence/1`, docs/courtevidence-v1.md)

- `evidence-py/`: the criterion candidate of the evidence law, pure Python
  3 over the `galeed/1` core above (`gev.py`, `gevlaw.py`). Its
  release digest is computed by `criterion-digest-evidence.sh` and named by
  §8.5 of that law.
- `HARNESS-EVIDENCE.md`: the command-line contract
  (`gev check|digest|recipient-check|pick-recipient`).
- `evidence-corpus/gen.py`: the corpus generator with hand-written
  predictions (241 cases); seed 1 is committed; the README carries the
  boundary list the law's §8.1 makes a precondition.
- `compare-evidence.py`: runs a candidate named by `GEV_CANDIDATE` and the
  criterion over the evidence corpus; without a candidate it checks the
  criterion against the predictions alone and says so. The manifest defines
  the corpus: every corpus path a case names must carry a file, except the
  paths the case lists as `absent`, which must carry none. A case passes
  where the two sides agree and the criterion's answer equals the prediction
  byte for byte (exit 2 and no output where the prediction is null); a file
  present or absent against the manifest, a corpus with no case and an
  unreadable manifest are disagreements. It prints the first sixty
  disagreements and exits 1 where any divergence or disagreement was found,
  0 otherwise.

Review records: `docs/galeed-v1-review.md`; the papers law's rounds are
recorded in `docs/galeed-papers-v1-review.md`, the roster law's in
`docs/galeed-roster-v1-review.md`, the evidence law's in
`docs/courtevidence-v1-review.md`.
