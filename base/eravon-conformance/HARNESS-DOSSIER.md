# erd convergence harness: command-line contract for eravon.dossier/1

The measuring interface shared by every independent implementation of
`eravon.dossier/1` (docs/eravon-dossier-v1.md). It defines nothing about
the law. Every implementation embeds conforming `eravon.link/1` and
`eravon.terms/1` cores and carries its own implementation of `galeed/1`
§4.3 (law §1, §7.5); the criterion here (`eravon-conformance/dossier-py/`)
embeds the two beside it and carries `erdentry.py`.

Output, exit codes, misuse, the sixteen-mebibyte bound, and the fault
object are as HARNESS-TERMS.md and HARNESS-LINK.md fix them. In this law
`E_DOSSIER_FILE` carries `"key":K` with `K` a file key of the law's §1
(`index`, naming the index file `dossier.json` itself; `listing`;
`receipt`; `entry`; or `attachment:<i>`, the element's index in
`files.attachments` spelled as an int), and `E_DOSSIER_DIGEST` carries a
file key other than `index`; `E_TRANSACTION`, `E_DOSSIER_COMPLIANCE`, and
`E_DOSSIER_ATTACHMENT` carry `index` where the law says;
`E_DOSSIER_LISTING`, `E_DOSSIER_RECEIPT`, `E_DOSSIER_ENTRY_FORM` carry
`inner`, the other law's fault with whatever it carries (a `galeed/1`
token under `E_DOSSIER_ENTRY_FORM` carries nothing, that law's `E_JUDGE`
and `E_SIG_FORM` included, HARNESS-TERMS.md's prefix rule being the terms
law's own), written without `ok` as HARNESS-TERMS.md's fault object says
of every carried fault; `E_DOSSIER_ORDER`
carries `inner`, the link law's verdict written as a fault object whose
`token` is the verdict name, with `inner` where it carries a fault, nested
as deep as the carrying goes.

## Arguments

- `<dir>`: a path the host's path resolution resolves, symbolic links
  followed, to a directory whose entries can be listed; anything else is misuse, and this is the
  one place a `<dir>` argument differs from HARNESS-TERMS.md's
  unreadable-path rule: the files inside the directory are decided by the
  law (a directory lacking a readable `dossier.json` answers
  `E_DOSSIER_FILE` with `"key":"index"`; a named entry the process cannot
  open, for want of permission on the entry or on the directory among the
  causes, answers `E_DOSSIER_FILE` with its key). Files are read by the
  exact name the index gives, relative to the directory, the name matched
  byte for byte against the directory's own entries (a host's name folding
  never reaches an answer; a symbolic link's target and `<dir>`'s own path
  resolve by the host's rules, its folding included), `dossier.json` included; each file is read by handing the host, unnormalised, the bytes of
  `<dir>`, then a `/` unless those bytes already end in one, then the
  name, so that every link that resolution follows, `<dir>`'s own
  included, counts against the host's link limit (32 links followed on
  macOS, 40 on Linux, the next refused), and the path's bytes against its
  path-length limit (1024 bytes on macOS, 4096 on Linux, the terminating
  NUL included, so that the longest path the host opens is 1023 or 4095
  bytes; macOS applies it also to the path as resolution rewrites it at
  each link it follows); an entry that is neither a regular file nor a
  symbolic link that resolution follows, through further links, to one is
  unreadable (a resolution past either limit among them; no corpus entry's
  resolution follows more than two links, but for the loop of two of
  `link_loop`, which no host resolves), and a link it so follows is read
  as that file; the index is
  read as `dossier.json`.

Executable name: `erd`. Commands:

## `erd check <dir>`

§2.6 then §3.1. Accepted:
`{"attachments":<int>,"buyerConsent":"secp256k1"|"call","candidate":<int>,"caseId":"0x<64>","entryId":"0x<64>"|null,"instance":"0x<40>","ok":true,"sellerConsent":"secp256k1"|"call","termsHash":"0x<64>"}`,
`attachments` the number of attachment references (0 where the member is
absent) and the two consent forms as §3.1's answer defines them.
Rejected: the fault object.
