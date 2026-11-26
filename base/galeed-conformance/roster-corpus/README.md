# roster corpus

The generated corpus of `galeed.roster/1` (docs/galeed-roster-v1.md), built
by `gen.py --seed N` from the law and HARNESS-ROSTER.md. Seed 1 is
committed; `../compare-roster.py` runs a candidate named by `GRO_CANDIDATE`
and the criterion `../roster-py/gro.py` over `manifest.json`, counts every
divergence and every disagreement with the generator's prediction, prints
the first sixty, and exits 1 where it found any.

- `pubs/`: one publication per file (its `document` bytes in the transport
  hex spelling and its `word`), exercised by `decode`.
- `logs/`: logs sets, each carrying the registry's publications and
  statuses and the succession book's declarations and accessions
  (`l001_chain1.json` is the one every resolution case reads; the others
  are misuse shapes, a set carrying members beyond the named ones
  (`extra_members`), an empty set, and two small books), exercised by
  `publication`, `resolve-identity`, `current-at`, `resolve-guild`, `judge`,
  `guild-card`, `resolve-judge`, `covenant`, `bench`, `candidates`.
- `bench/`: values for `bench`.
- `query/`: queries for `candidates`.
- `quotes/`: `quotes` objects for `quote`.

`manifest.json` lists every case as `{cmd, args, path, note, predicted}`,
or as `{argv, path, note, predicted}` where `argv` is the whole argument
vector after the executable name (an empty `argv` runs `gro` alone); paths
are relative to this directory; a case whose `predicted` is `null` is
harness misuse and the expected answer is exit code 2 with no output. The
seed draws the addresses and the digests and never the case list. The
prediction is the generator's own reading, written by hand from the text,
and never authoritative: where two implementations agree with each other
and disagree with it, the prediction is wrong.

## Closed boundaries (law §10.1's precondition)

Each boundary names its witnesses by case slug (the file name after its
number) or, for a resolution case, by a phrase of the case's note as the
manifest carries it. A boundary with one reachable side is listed with
that side alone.

### §1 of a kind, `candidateOf`

| boundary | witnesses |
|---|---|
| a publication failing step 1 (the word), 2 (the reader) or 3 (`kind`) is of every kind | "unreadable bytes are of every kind" (as roster, as verifier, as guild, as covenant), "and of the card kind", "unreadable bytes after a good card are the card candidate", "CARD_UNREAD with E_KIND", "a publication whose hash fails is of every kind", "bytes that fail step 1 are of every kind, so they are the roster candidate too", "unreadable bytes are of every kind, so they are the candidate; unread at step 2", "a kind that is not a string is of every kind; E_KIND" (as guild, as judge, as covenant), "unreadable bytes before the roster are the covenant and card candidates of every kind", "a wrong-word publication before the roster" |
| a kind under this prefix: this version, another version (a later one, the bare prefix, a trailing space) | "a later roster version is of the roster kind and is the candidate", "a later version of the kind" (covenant), "a later version; unread with E_UNKNOWN_KIND" (judge), "a judge document that fails its table is the judge candidate", "the bare prefix is a kind under the prefix of another version", "the literal with a trailing space is another version" |
| a kind under another reserved prefix is of another kind | "a judge document from the guild's address is of another kind", "a verifier service record is of another kind", "an address with rosters and a judge document", "the same address asked for its roster candidate" (a card beside a roster), "a card before the roster; each reading takes its own kind", "a card is of another kind than a covenant" |
| a kind under no reserved prefix is of another kind | "a kind under no reserved prefix is of another kind for every reader", "the prefix without its slash is a kind under no reserved prefix", "a former succession record's kind sits under no reserved prefix", "a former succession record from the guild's address is of another kind", "a former succession record is of another kind; no document" |
| the last by `(blockNumber, logIndex)`, never by file order; the largest block and log index | "candidateOf for the card kind is the last card, by (blockNumber, logIndex)", "two publications in one block are ordered by logIndex", "blockNumber and logIndex at 2^53 - 1" |

### §2.1 decoding

| boundary | fails | passes |
|---|---|---|
| step 1, the word | `hash_mismatch`, `hash_of_other_bytes`, `hash_before_json` (before step 2) | every accepted case |
| step 2, the reader: UTF-8 (invalid, overlong, encoded surrogate, above U+10FFFF, truncated), grammar, numbers (fraction, sign, -0, leading zero, exponent in either case, 2^53, eighteen digits), repeats, surrogate escapes (lone high, lone low, high then letter, two highs), depth, BOM, the empty document | `json_invalid_utf8`, `json_utf8_overlong`, `json_utf8_encoded_surrogate`, `json_utf8_above_max`, `json_utf8_truncated`, `json_trailing_garbage`, `json_nan`, `json_float`, `json_negative`, `json_negative_zero`, `json_leading_zero`, `json_exponent`, `json_exponent_upper`, `json_two_pow_53`, `json_eighteen_digits`, `json_dup_key`, `json_dup_key_escaped`, `json_lone_surrogate`, `json_high_surrogate_then_letter`, `json_lone_low_surrogate`, `json_two_high_surrogates`, `json_depth_129`, `json_bom`, `hash_empty_document` | `json_max_int`, `json_surrogate_pair`, `json_depth_128`, `json_pretty_printed`, `json_ws_around`, `json_prose_controls`, `json_non_ascii_key`, `json_raw_non_ascii`, `json_escaped_kind_and_key`, `json_escape_upper_digits` |
| step 2, root an object | `json_root_array`, `json_root_string` | |
| step 3, `kind` | `kind_absent`, `kind_int`, `kind_null`, `empty_object` | |
| step 4, the four kinds and the unknown kind (a later version, the bare prefix, a version spelled with a leading zero, a trailing space, the prefix without its slash, the former succession record's kind) | | `unknown_kind_v2`, `unknown_kind_empty`, `unknown_kind_verifier`, `unknown_kind_other_family`, `unknown_kind_judge_v2`, `unknown_kind_guildamanah_v2`, `unknown_kind_guildcard_v2`, `succession_record_unknown_kind`, `succession_prefix_not_reserved`, `kind_case`, `kind_bare_prefix`, `kind_version_zero`, `kind_version_leading_zero`, `kind_trailing_space`, `kind_no_slash` |
| §2.2 kept members, bytewise over UTF-8; a member carried whole (`quotes`) with keys and nested names of any scalar value in UTF-8 order | | `roster_full`, `roster_expires_name_terms_unknown`, `guildcard_judge_members_unknown`, `judge_extra_members`, `guildamanah_full`, `json_key_order_utf8_bytes`, `json_non_ascii_key`, `roster_pool_member_is_extension`, "quotes is kept whole; keys and nested names of any scalar value come out in UTF-8 byte order" (`covenant` and `resolve-guild`), "a judges element's extra members may nest containers" |

### §3 the roster

| boundary | fails | passes |
|---|---|---|
| `guild` hex20 | `roster_guild_absent`, `roster_guild_checksummed`, `roster_guild_short`, `roster_guild_int`, `roster_guild_no_prefix` | `roster_minimal` |
| `chainId`, `issued` int | `roster_chain_absent`, `roster_chain_string`, `roster_issued_absent`, `roster_issued_bool` | `roster_minimal` |
| `expires`, `name`, `terms`: no members of the table, carried beyond it whatever their value (one-sided) | | `roster_expires_name_terms_unknown`, `roster_expires_null`, `roster_expires_string`, `roster_name_int`, `roster_name_empty`, `roster_terms_array`, `roster_expires_and_name_untested`, `json_surrogate_pair`, `json_prose_controls`, `json_raw_non_ascii` |
| `judges` and its two element forms | `roster_judges_absent`, `roster_judges_object`, `roster_judges_element_int`, `roster_judges_element_no_addr`, `roster_judges_element_addr_upper`, `roster_judges_element_null` | `roster_full`, `roster_judges_repeated`, `roster_judges_since_untested`, `roster_judges_document_order` |
| `bench` shape, its two members, the verifier object's three members | `roster_bench_null`, `roster_bench_array`, `roster_bench_no_protocols`, `roster_bench_no_verifiers`, `roster_bench_protocol_hex20`, `roster_bench_verifier_no_name`, `roster_bench_verifier_name_int`, `roster_bench_verifier_protocols_absent`, `roster_bench_verifier_protocol_bad`, `roster_bench_verifier_upper`, `roster_bench_verifier_int`, `roster_bench_verifier_object_no_addr`, `roster_bench_verifier_object_addr_upper`, `roster_bench_verifier_object_addr_int` | `roster_bench_extra_member`, `roster_bench_empty`, `roster_bench_verifier_views`, `roster_bench_verifier_name_any_scalar` |
| §3.2 order, every adjacent pair | `roster_order_guild_before_chain`, `roster_order_chain_before_issued`, `roster_order_issued_before_judges`, `roster_order_judges_before_bench`; no step reads `expires` or `terms`: `roster_order_issued_then_judges`, `roster_terms_beside_bad_judges` | |

### §3a the judge document

| boundary | fails | passes |
|---|---|---|
| `judge` hex20 | `judge_judge_absent`, `judge_judge_upper` | `judge_minimal` |
| `issued` int | `judge_issued_absent`, `judge_issued_string` | `judge_minimal` |
| `amanah`: object, `entry_id` present and hex32, members beyond it untested | `judge_amanah_absent`, `judge_amanah_string`, `judge_amanah_no_entry_id`, `judge_amanah_entry_id_hex20`, `judge_amanah_entry_id_upper` | `judge_minimal`, `judge_extra_members` |
| `ledger`: object, `root` hex20, `basis` a `galeed/1` basis (that law's §9.4 rules on the basis member: form and arity of the basis and of each chains object and class item, `fromBlock` ≤ `toBlock`, chains ascending by `(chainId, fromBlock)`, windows of one chain disjoint and, where they touch, differing, `senders` ascending without repeats, `registries` addresses then class items each ascending without repeats), members beyond the two untested | `judge_ledger_absent`, `judge_ledger_array`, `judge_ledger_no_root`, `judge_ledger_root_short`, `judge_ledger_no_basis`, `judge_basis_not_object`, `judge_basis_second_member`, `judge_basis_chains_missing`, `judge_basis_senders_unsorted`, `judge_basis_range`, `judge_basis_windows_touch_equal`, `judge_basis_registry_upper`, `judge_basis_class_item_extra`, `basis_touching_equal_lists`, `basis_overlap_one_block`, `basis_nested_window`, `basis_class_codehash_short`, `basis_class_codehash_upper`, `basis_class_codehash_hex20`, `basis_class_codehash_int`, `basis_registries_not_array`, `basis_senders_not_array`, `basis_sender_not_hex20`, `basis_equal_from`, `basis_chains_out_of_order`, `basis_duplicate_sender`, `basis_senders_out_of_order`, `basis_class_before_address`, `basis_class_out_of_order`, `basis_class_duplicate`, `basis_registry_hex32_string`, `basis_class_extra_member`, `basis_chain_extra_member`, `basis_chain_missing_senders`, `basis_from_above_to`, `basis_block_not_int` | `judge_minimal`, `judge_basis_two_chains`, `judge_root_differs_from_judge`, `judge_extra_members`, `basis_empty_chains`, `basis_from_equals_to`, `basis_touching_differ_senders`, `basis_touching_differ_registries`, `basis_equal_windows_two_chains`, `basis_gap_one_block_equal_lists`, `basis_max_ints`, `basis_registries_address_then_class`; through `judge`: "a basis with no chains is a basis", "two windows of one chain a block apart with equal lists" |
| `name`, `home`, `note_md` | `judge_name_int`, `judge_home_null`, `judge_note_array` | `judge_full`, `judge_name_empty` |
| §3a.2 order, every adjacent pair | `judge_order_judge_before_issued`, `judge_order_issued_before_amanah`, `judge_order_amanah_before_ledger`, `judge_order_ledger_before_name`, `judge_order_name_before_home`, `judge_order_home_before_note` | |

### §3b the guild covenant

| boundary | fails | passes |
|---|---|---|
| `guild` hex20, `chainId`, `issued`, `version` int | `guildamanah_guild_absent`, `guildamanah_guild_upper`, `guildamanah_chain_string`, `guildamanah_issued_absent`, `guildamanah_version_string`, `guildamanah_version_absent` | `guildamanah_minimal` |
| `text_md` a string; `sha256` over its UTF-8 scalar values | `guildamanah_text_absent`, `guildamanah_text_int` | `guildamanah_text_empty`, `guildamanah_text_non_ascii_sha`, `guildamanah_full` |
| `venues`: array of objects with the three members | `guildamanah_venues_object`, `guildamanah_venue_no_codehash`, `guildamanah_venue_contract_hex32`, `guildamanah_venue_string`, `guildamanah_venue_no_chainid`, `guildamanah_venue_chainid_string` | `guildamanah_venues_empty`, `guildamanah_full` (members beyond the three not carried) |
| `quotes`: an object, its members untested | `guildamanah_quotes_array`, `guildamanah_quotes_null` | `guildamanah_quotes_empty`, `guildamanah_full` (a key outside §5.10's forms kept) |
| §3b.2 order, every adjacent pair | `guildamanah_order_guild_before_chain`, `guildamanah_order_chain_before_issued`, `guildamanah_order_issued_before_version`, `guildamanah_order_version_before_text`, `guildamanah_order_text_before_venues`, `guildamanah_order_venues_before_quotes` | |

### §3c the guild card

| boundary | fails | passes |
|---|---|---|
| `guild` hex20 | `guildcard_guild_absent`, `guildcard_guild_upper`, `guildcard_guild_null` | `guildcard_minimal` |
| `issued` int | `guildcard_issued_absent`, `guildcard_issued_string` | `guildcard_minimal`, `guildcard_issued_max` |
| `name`, `home`, `note_md`, each a string of any scalar values | `guildcard_name_int`, `guildcard_home_null`, `guildcard_note_array` | `guildcard_full`, `guildcard_name_empty`, `guildcard_home_any_string` |
| members beyond the table, a judge document's `amanah` and `ledger` among them | | `guildcard_judge_members_unknown` |
| §3c.2 order, every adjacent pair | `guildcard_order_guild_before_issued`, `guildcard_order_issued_before_name`, `guildcard_order_name_before_home`, `guildcard_order_home_before_note` | |

### §4 the succession book (cases named by their note; `logs/l001_chain1.json` unless named)

| boundary | fails | passes |
|---|---|---|
| the current key's two conditions (a declaration binds only from the current key of its record, an accession counts only at a current holder; each implies the other, so the two are witnessed together, by one case that fails both) | "a key that left declares and its named key accedes: both count for nothing" | "a declaration and the named key's accession move the record" |
| a declaration's `to`: nonzero, not the emitter, not entered (by acceding or by declaring) | "a declaration to the zero address binds nothing", "a self-declaration does not make its emitter enter", "a key that entered by acceding is never named again", "a declaration naming a key that entered binds nothing and leaves the emitter's pending key" | "a declaration no accession took up moves nothing" (pending), "two holders name one key; the key takes up the second" (a key pending elsewhere has not entered) |
| a declaration that binds nothing enters nothing | | "the emitter does not enter, so it can take up a record later", "a declaration naming an entered key does not make its emitter enter" |
| declaring again replaces the pending key; a declaration that binds nothing replaces nothing; an accession takes up `holder`'s last binding declaration, the one that made the acceding key `holder`'s pending key, and no stranger's declaration naming that key | "the replaced declaration and the late one are attempts", "a declaration replaced by a later one is an attempt" | "declaring again replaces the pending key", "a non-binding declaration from the current key leaves its pending key", "two declarations name one key, the second replacing the first", "a stranger naming the new key does not make it enter; the new key takes up the record", "the stranger's declaration stays its own attempt" |
| an accession counts only from a key that has not entered, pending at a current holder | "an accession with no declaration pending counts for nothing", "an accession by a key that has entered counts for nothing", "the stale accession counts for nothing", "a replaced key's accession after the record moved counts for nothing", "its later accession of the first counts for nothing" | every move |
| the accession sets `holder`'s pending key to none | one side only: a holder whose pending key was taken up is no longer current, so nothing can observe the pending key it no longer has | every move |
| the zero address as an emitter and a root | | "the zero address is a key like any other as an emitter", `list_zero_root` |
| the walk by `(blockNumber, logIndex)`, `blockNumber` first, never by file order | "an accession before the declaration in the same block, by logIndex", "blockNumber is compared first" | "records are walked by (blockNumber, logIndex), never by file order" |
| the accession's `root` member is read for its form alone | | "the root member of an accession is read for its form and by no step of the replay" |
| book records at the top of the range | | "book records at block and log index near 2^53 - 1" |
| a declaration alone, a declaration with its accession (small books) | `logs/*_only_declaration.json` | `logs/*_declaration_and_accession.json` |

### §5 resolution (cases named by their note; all read `logs/l001_chain1.json`)

| boundary | witnesses |
|---|---|
| §5.1 `active`: absent, false, resumed, tie within a block; standing at the address given | "an address the logs never name", "latest status false", "false then true resumes", "two statuses in one block", "a status without any publication", "a retired judge's document reads", "publication reads the address it is given" |
| §5.2 `currentAt`: the records at the position unread, one position past it, `blockNumber` compared first (the next block at log index 0, the block before at a greater log index), the records later in the position's block unread, a declaration's own position, a key asked before it acceded, the top of the range (a record at the position asked, every record below the greatest position, and a record at the greatest position, which `current` reads and no `current-at` does, in its own logs set `top_position`), before every record, an address never met, a holder whose pending key took up another record; the harness's spellings of the position and the address | "at the accession's own position the accession is unread", "one position past it the record has moved", "the next block at log index 0 reads the accession", "the block before, at a greater log index, does not", "the records later in the position's own block are unread", "at a declaration's own position", "the second accession's block at log index 0", "a key asked before it acceded", "the root asked before the second accession", "near the top of the range, a record at the position asked is unread", "at the greatest position every record below it is read", "current reads a record at the greatest position", "every current-at leaves a record at the greatest position unread", "the acceding key asked at the greatest position", "at the top of the range", "before every record", "an address the book never met", "a holder whose pending key took up another holder's record keeps its own", "the other holder's record moved to the key"; misuse: a leading zero, a sign, a plus sign, a fraction, hexadecimal (each in both arguments), an empty argument, above 2^53 − 1 (both arguments), an uppercase address, an unreadable logs file, a missing argument, an argument beyond the list (through `current-at`) |
| §5.3 the record's keys (a pending key none of them, witnessed through the bench and `deny`: `list_key_with_pending`, `pointer_verifier_with_pending`, `deny_pending_key`); `final`, `trail` from the address asked, `attempts` of every key of the trail and of `final` in walk order across keys, the declarations accessions took up excluded (an accession takes up its holder's last binding declaration, the one before it and a stranger's naming of the same key staying attempts, in its own logs set `redeclared_twice`), the final key's pending one included; no bound | "the final key's pending declaration is an attempt, its named key no key of the record", "asked at the root of that record", "an address the book never met is its own final", "the trail runs from the address asked; the current key's trail is empty", "asked in the middle of a record", "asked at the root", "attempts of every key of the trail and of the final key, in walk order across keys", "asked in the middle, the attempts of the keys from there on", "asked at the current key", "sixty-five hops resolve; the walk needs no bound", "sixty-four hops", "one hop", "the end of the record is its own final", "a key named by declarations it never took up is its own final", "the would-be acceder entered nothing", "the stranger's declaration stays its own attempt", "that key's own record", "a key that declared is its own final with its pending declaration", "the accession takes up the holder's last binding declaration", "a stranger that named the same key keeps its own record" |
| §5.4 step 1 never stops | "step 1 never stops; step 2's standing decides", "the guild's record moved to an address that published no roster" |
| §5.4 step 2 retired, resumed, standing at G | "retired", "resumed", "resumed within one block", "standing is read at the final address", "a retired origin does not retire the successor", "retired is decided before no document", "retired before mismatch" |
| §5.4 step 3 | "no publication", "the successor published nothing", "a covenant alone is no roster", empty logs set |
| §5.4 step 4: a decoding fault, a later roster version; publications of other kinds moving nothing; unreadable bytes declining the guild | "the roster candidate fails decoding", "a later version of the kind; the older roster is not read", "a former succession record from the guild's address", "a judge document from the guild's address", "a kind under no reserved prefix", "a verifier's service record from the guild's address", "unreadable bytes are of every kind and are the roster candidate" |
| §5.4 step 5 both mismatches and their order | "names another guild", "names another chain", "guild mismatch before chain mismatch", "chain mismatch", "gg4's roster names chain 1" |
| §5.4 no step reads a time | "a roster that writes expires resolves and carries it beyond the table", "issued is carried as written" (2^53 − 1) |
| §5.4 step 6 rows | "the later version supersedes", "every row resolves; the bench dedups", "the successor's roster is read, the trail reported" |
| §5.4 step 7 verifier rows, the bench as every key of each listed verifier's record (the keys before the listed address included), protocols as written, no bench | "verifier rows carry name and protocols", "a moved verifier contributes its final and its trail", "the bench is every key of the record of the listed verifier", "no bench member resolves to the empty bench" |
| §5.4 step 8 `issued`, kept members, the card member (`CARD_NONE`, `CARD`, `CARD_MISMATCH`, `CARD_UNREAD` with a table token, with `E_UNKNOWN_KIND`, with `E_JSON` and `E_HASH` behind a valid roster; read at the final address), the covenant member: `null`, the current covenant (its `quotes` whole), every stop (a table token, a mismatch, a later version, and the decoding stops `E_JSON`, `E_HASH`, `E_KIND` behind a valid roster) | "issued is carried as written", "name, terms and the extension members are members beyond the table", "no card, no covenant", "the card is the guild's last, with its word and block", "a card naming another guild", "the last card fails its table", "the last card is of a later version", "unreadable bytes before the roster are the covenant and card candidates", "a wrong-word publication before the roster", "and the card read at the successor, which published none", "the covenant is the guild's current one (the later of two)", "the covenant's quotes reach the guild answer whole", "the covenant names another guild", "the covenant is for chain 2", "the last covenant is of a later version", "the covenant fails its table", "a kind that is not a string is of every kind; E_KIND" |
| §5.5 every row of B | `null`, `pointer_ok`, `pointer_no_bench_member`, `pointer_other_chain`, `pointer_retired`, `pointer_no_document`, `pointer_moved_to_nothing`, `pointer_roster_writes_expires`, `pointer_moved_guild`, `pointer_guild_with_verifier_objects`, `pointer_guild_covenant_stop`, `pointer_guild_card_unread`, `pointer_chain_above_range`, `pointer_middle_verifier`, `list_ok`, `list_empty`, `list_bad_element`, `list_element_int`, `list_repeated`, `list_long_record`, `list_mid_record`, `list_middle_with_root`, `list_final_key`, `list_zero_root`, `list_key_with_pending`, `list_pending_key`, `list_key_moved_after_roster`, `list_record_moved_at_greatest_position`, `pointer_verifier_with_pending`, `value_number`, `value_object`, `value_true`; the `B` absent half is unreachable through the harness |
| §5.6 union, dedup over both halves, deny by any key of a row's record (the final key, a trail key, a key in the middle of a long record, a key before the listed address, the root of a listed final key, a key that acceded after the roster, the zero address as a root) and by no other address (a pending key, the key of a replaced declaration), two rows of one record, allow after deny, deny does not reach a guild's trail, a row whose record is the guild's own, refused guilds contribute nothing, repeated pointers, a pool guild, a card guild, a guild whose covenant member is a decoding stop | `union_dedup`, `deny_by_trail`, `deny_by_final`, `deny_mid_trail`, `deny_predecessor_of_listed`, `deny_successor_of_listed`, `listed_middle_not_denied`, `old_and_successor_listed`, `old_and_successor_denied`, `allow_after_deny`, `allow_only`, `deny_guild_own_trail`, `deny_pending_key`, `deny_root_of_listed_final`, `deny_key_acceded_after_roster`, `deny_replaced_declaration_target`, `deny_zero_root`, `guild_own_row_contributes`, `deny_guild_own_key`, `moved_rows`, `unresolved_guild`, `duplicate_pointer`, `moved_guild_pointer`, `pool_guild_candidates`, `card_guild_candidates`, `covenant_stop_from_unreadable_bytes` |
| §5.6 `E_POINTER` and `E_POINTER_CHAIN` | `bad_pointer`, `chain_mismatch_pointer`, `pointer_pieces` |
| §5.7 `judgeOf`: `JUDGE`, `JUDGE_NONE`, `JUDGE_MISMATCH`, `JUDGE_UNREAD` with a table token, with `E_UNKNOWN_KIND`, with each decoding token (`E_JSON`, `E_HASH`, `E_KIND`); follows no record; a document beside rosters; standing unread; the basis edges a document carries | "a judge's document, read at its own address", "no publication", "the document's judge is another address", "the last judge document fails its table", "a later version; unread with E_UNKNOWN_KIND", "unreadable bytes are of every kind, so they are the candidate; unread at step 2", "the last judge document fails its hash; JUDGE_UNREAD with E_HASH", "s2.1 step 3 as the judge candidate: JUDGE_UNREAD with E_KIND", "a document that fails at judge", "a former succession record is of another kind; no document", "judgeOf follows no record", "an address with rosters and a judge document", "a retired judge's document reads", "a basis with no chains is a basis", "two windows of one chain a block apart" |
| §5.8 `resolveJudge`: never moved, moved (trail and the final address's document), moved to a key with no document, active read at the final address, a long record, nothing at all, another chain | "a judge that never moved", "the judge moved", "the judge moved to a key that published no document", "a false status at the origin binds no reader; active is read at the final key", "a false status at the final key", "a long record resolves", "an address with nothing resolves to itself with JUDGE_NONE", "on chain 2: the book's records of the logs set are read as they are" |
| §5.9 `covenantOf`: `null`, the current one, guild mismatch, chain mismatch (and the same covenant on its own chain), both mismatches (guild first), a later version, a hash fault, a table token, `E_KIND`, unreadable bytes, follows no record | "no covenant published", "the guild's current covenant, the later of two", "a covenant with the required members alone", "guild mismatch", "chain mismatch", "on chain 2: the covenant names chain 2 and reads", "guild mismatch is decided before chain mismatch", "a later version of the kind", "the last covenant fails its hash", "the table's token", "s2.1 step 3 as the covenant candidate: a stop with E_KIND", "unreadable bytes are the candidate of every kind", "covenantOf follows no record: the origin's own covenant, never the successor's" |
| §5.10 `quoteOf`: the five keys in order, the token keys skipped without a token, `QUOTE_NONE` (none of the five, the empty object), `E_QUOTE_FORM` (uppercase, odd, int, no prefix) and its precedence over a later good key, untested keys, the chain key's spelling, bytewise keys, the empty quote | `address_token_first`, `address_without_token`, `code_token`, `code_without_token`, `chain_alone`, `none`, `empty_object`, `empty_quote_present`, `form_fault_uppercase`, `form_fault_odd`, `form_fault_int`, `form_fault_no_prefix`, `form_fault_shadows_later_good`, `token_key_skipped_without_token`, `other_chain_only`, `untested_keys_data`, `chain_key_spelling`, `large_chain`, `case_of_key_address`, `guild_quotes_from_logs`, `guild_quotes_code_token` |
| §5.11 `guildCardOf`: `CARD`, `CARD_NONE`, `CARD_MISMATCH`, `CARD_UNREAD` with a table token, with `E_UNKNOWN_KIND`, with `E_JSON` and `E_KIND`; follows no record of the book | "the last card of the address", "no card", "a card naming another guild", "a card failing its table", "a later card kind", "unreadable bytes after a good card are the card candidate", "a kind that is not a string is of every kind; CARD_UNREAD with E_KIND", "guildCardOf reads the address it is given and follows no record of the book", "the successor published no card", "the final address's card names the origin: CARD_MISMATCH", "the card is read at the final address, whatever the origin's card", "a kind that is not a string, before the roster, stops the card and the covenant" |

### §6 the pointer

`pointer_checksummed`, `pointer_leading_zero_chain`, `pointer_prefix_case`,
`pointer_trailing_space`, `pointer_chain_zero`, `pointer_chain_above_range`
(through `bench`) and `pointer_pieces` (through `candidates`: the missing
colon, an empty chain id, a non-digit chain id, 2^53, a short address,
2^53 − 1, a trailing field, an uppercase address, leading whitespace).

### HARNESS-ROSTER

Argument spellings: `chainId` `01`, `0`, above 2^53 − 1, at 2^53 − 1,
`+1`, `x`; addresses uppercase, `0X`, short; `codeHash` hex20, `token`
uppercase; the kind word outside the five, in another case, and the former
`succession`; argument counts for every command, a clock argument beyond
the lists of `resolve-guild`, `bench`, and `candidates`; the arguments of
`resolve-judge` in another order and a fourth one beyond its list; no
command and an unknown command.
File forms: `misuse_*` cases under `pubs/` and `quotes/`; under `logs/`,
pairs shared across every two arrays and within each array, each array
absent or of another type, a declaration and an accession that are no
objects, a declaration's `to`, `emitter`, and `blockNumber`, an accession's
`root`, `holder`, and `logIndex` out of form, a publication without `document`, a status that is no object or whose `blockNumber` is absent or a string, a root that is no object (`root_array`), the two-array form; the
`bench/` unreadable file, the `query/` misuse cases; open members ignored
(`publication_extra_member`, `extra_members`); transport digits of either
case (`document_transport_upper_digits`).
