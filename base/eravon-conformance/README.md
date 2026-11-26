# eravon-conformance

The criterion layer of the three ERAVON document laws: what an
implementation is measured against, and the tools that measure it.
Nothing here is an implementation for use; the implementations below the
laws (the seller's client, the buyer's client, a judge's desk) are born
elsewhere and read against this.

The laws read `galeed/1` through implementations their own cores carry:
the terms core carries the forms, canonical form, `keccak256` and
secp256k1 (`eravon.terms/1` §10.4), and the dossier core the entry test of
`galeed/1` §4.3 (`eravon.dossier/1` §7.5); no core here embeds a `galeed/1`
core or names its digest.

## The terms law (`eravon.terms/1`, docs/eravon-terms-v1.md)

- `terms-py/`: the criterion candidate, pure Python 3 (`ert.py`,
  `ertlaw.py`, `ertcrypto.py`, and `ertcanon.py`, the canonical form of
  `galeed/1` §3 with the harness's input reader). `criterion-digest-terms.sh`
  computes its release digest (sha256 over the four files' digest lines);
  §10.5 of the law is where `naming.py` names it, and until then the text is the criterion.
- `HARNESS-TERMS.md`: the command-line contract every candidate implements
  (`ert check|quote|offer|order|action|sign-offer|sign-order|sign-action|verify-offer|verify-order|verify-action|fee`).
- `terms-corpus/gen.py`: the corpus generator with hand-written
  predictions; seed 1 is committed (4,239 cases, 3,656 of them the `galeed/1` corpus's `check` inputs as document blocks); the README carries the
  boundary list the law's §10.1 makes a precondition.

## The link law (`eravon.link/1`, docs/eravon-link-v1.md)

- `link-py/`: the criterion candidate over the terms core (`erl.py`,
  `erllaw.py`, the book's replay and §6 identity among its predicates);
  `criterion-digest-link.sh`; §11.5 is where `naming.py` names it.
- `HARNESS-LINK.md`: `erl check|bind|link-check|encode|decode|resolve-link|receipt-check|receipt-encode|receipt-decode|bind-receipt|pack-check|settle-check|bind-settle|place|pay-check|history`.
- `link-corpus/gen.py`, seed 1 committed (4,505 cases, 3,656 of them the `galeed/1` corpus's `check` inputs as listings), with its boundary
  list.

## The dossier law (`eravon.dossier/1`, docs/eravon-dossier-v1.md)

- `dossier-py/`: the criterion candidate over the link core (`erd.py`,
  `erdlaw.py`, and `erdentry.py`, its own `galeed/1` §4.3);
  `criterion-digest-dossier.sh`; §7.5 is where `naming.py` names it.
- `HARNESS-DOSSIER.md`: `erd check`.
- `dossier-corpus/gen.py`, seed 1 committed (1,197 cases, 854 of them the `galeed/1` corpus's `check` inputs that pass its §3.5 as entry files), with its
  boundary list.

## Shared tooling

- `gencommon.py`: the generators' shared helpers, with their own typed
  data, CREATE2 and action-message code (a third opinion) and the GALEED
  base's `galeed/1` core (`../galeed-conformance/impl-py`) for canonical
  form, secp256k1 and the entry digest; no prediction passes through a
  criterion.
- `compare.py terms|link|dossier [corpus-dir]`: runs a candidate named by
  `ERT_CANDIDATE`, `ERL_CANDIDATE`, or `ERD_CANDIDATE` and the criterion
  over the corpus and prints every divergence and every disagreement with
  the generator's prediction; without a candidate it checks the criterion
  against the predictions alone and says so.
- `bcov.py terms|link|dossier <out.json>`: branch-arm witnessing of a
  criterion over its corpus (a measure, never a criterion).
- `naming.py <date> <fuzz-note>...`: the naming act, in the order the
  cores nest.

A commit that changes a form column of a member table, a member of a
binding's comparison list, or a law's §1, names in the same commit the
boundary it adds in the corpus README, or says the clause states no
boundary; the README diff is the reviewer's checklist.

Review records: `docs/eravon-terms-v1-review.md`,
`docs/eravon-link-v1-review.md`, `docs/eravon-dossier-v1-review.md`.
