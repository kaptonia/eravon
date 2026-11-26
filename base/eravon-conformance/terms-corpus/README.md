# terms corpus

The generated corpus of `eravon.terms/1` (docs/eravon-terms-v1.md), built
by `gen.py --seed N` from the law and HARNESS-TERMS.md, with the shared
helpers in `../gencommon.py`. Seed 1 is committed; `../compare.py terms`
runs a candidate named by `ERT_CANDIDATE` and the criterion
`../terms-py/ert.py` over `manifest.json` and prints every divergence and
every disagreement with the generator's prediction.

- `docs/`: terms documents and near-documents for `check`, and the
  documents the derivation, signature, and action cases read; the
  `gl1_*` documents carry, each as its block, one input of the `galeed/1`
  corpus's `check` set (`../../galeed-conformance/corpus/canon/`, 3656 at
  that corpus's committed seed, the same bytes under every seed here).
- `sigs/`: signature values for `verify-order` and `verify-action`; the
  `reader_*` files witness the `<sig.json>` reader HARNESS-TERMS cites
  (`galeed-conformance/HARNESS.md`, `gl1 audit`), each of its refusals once
  as misuse (a member twice, a fraction, a negative, 2^53, `NaN`, depth 129,
  a lone surrogate, a byte order mark) and three neighbours it admits
  (`E_SIG_FORM`).
- `quote` and `fee` cases carry their arguments in `argv` and no file.

`manifest.json` lists every case as `{cmd, args, path, note, predicted}`,
or as `{argv, path, note, predicted}` where `argv` is the whole argument
vector after the executable name (an empty `argv` runs `ert` alone);
paths are relative to this directory; a case whose `predicted` is `null`
is harness misuse and the expected answer is exit code 2 with no output.
The seed draws the keys and the digests, and never the case list. The
prediction is the
generator's own reading, written by hand from the text, and never
authoritative: where two implementations agree with each other and
disagree with it, the prediction is wrong.

## Closed boundaries (law §10.1's precondition)

### §2 the document (`check`)

| boundary | fails | passes |
|---|---|---|
| `galeed/1` §1's hex forms on each refusal that section names (an uppercase digit, an uppercase or absent prefix, another prefix, another length), on a hex20, a hex32 and a hex65 a predicate reads | `logic_upper`, `payout_upper`, `fund_upper`, `payout_prefix_upper`, `payout_prefix_other`, `payout_prefix_absent`, `goodshash_upper`, `goodshash_short`, `goodshash_prefix_upper`, `goodshash_prefix_absent`, `sig_prefix_upper`, `sig_prefix_absent`, the uppercase and 66-byte signatures | every accepted case |
| §2.3 `series`, where present, exactly `id` (nonzero hex32), `stage` (int, at least 1), `stages` (int, at least `stage`); §2.4's order, `compliance_md` before `series`; the twenty-three keys (each member missing once, the empty object, `null` as a member present, each member's form broken once) | `series_array`, `series_null`, `series_empty`, `series_missing_id`, `series_missing_stage`, `series_id_zero`, `series_id_short`, `series_id_hex20`, `series_id_upper`, `series_stage_bool`, `series_stages_string`, `series_stages_bool`, `series_stage_zero`, `series_stage_string`, `series_stages_below_stage`, `series_missing_member`, `series_extra_member`, `order_compliance_before_series` | `ok_series`, `ok_series_last_stage` (`stages` equal to `stage`), every case without `series` |
| §2.2 UTF-8, one opening fence, a closing fence after it | `utf8`, `utf8_in_block`, `no_block`, `crlf`, `fence_with_space`, `fence_indented`, `two_openings`, `two_openings_one_block`, `two_openings_unclosed`, `utf8_no_block`, `utf8_two_openings`, `utf8_unclosed`, `unclosed`, `close_with_space`, `close_before_open`, `unclosed_before_outside` | `ok_no_trailing_newline`, `ok_blank_lines_outside` |
| §2.2 every line outside the fences empty (`E_TERMS_OUTSIDE`), after the fence tests and before the block | `outside_before`, `outside_after`, `outside_space_line`, `outside_cr_line`, `outside_second_closing`, `outside_near_fence`, `outside_before_canonical`, `outside_before_root` | `ok_blank_lines_outside`, `ok_no_trailing_newline` |
| `canonical(block)`, its tokens, and canonical before the root (`canon_root_pretty`); `galeed/1` §3 as this core carries it (law §10.1, §10.4), measured by that law's own witnesses: every input of the `galeed/1` corpus's `check` set as the block of a document, predicted from that corpus's own prediction (an input that is no UTF-8 `E_TERMS_UTF8`, §2.4's first test over the whole document; a token of §3.5 carried over; a canonical value then `E_TERMS` at the root or `E_TERMS_MISSING` at the members), so that every boundary of `galeed/1` §3 that corpus witnesses, the order of its tests among them, is witnessed here | `canon_empty_block`, `canon_two_lines`, `canon_pretty`, `canon_unsorted`, `canon_dup_key`, `canon_key_charset`, `canon_value_charset`, `canon_value_charset_hash`, `canon_number`, `canon_json`, `canon_depth`, `canon_trailing_comma_object` (a comma before an object's closing brace, which that corpus reaches on no input), the `gl1_*` inputs that fail §3.5 | every accepted case, the `gl1_*` inputs that pass §3.5 (then refused at the root or the members) |
| root an object | `root_array`, `root_string` | |
| required (seventeen), then closed (twenty-three keys); a key that is exactly `_md` is prose and then beyond the list (§1) | `missing_amount`, `missing_buyeropen`, `missing_fund`, `missing_and_extra`, `closed`, `closed_milestones`, `closed_grant`, `closed_compliance_without_suffix`, `closed_md_bare`, `closed_before_v` | `ok_minimal` |
| `v`, `chainId`, `logic`, `sellerKey`, `sellerPayout`, `sellerFund`, `token` | `v_2`, `v_int`, `chain_string`, `chain_bool`, `logic_zero`, `logic_upper`, `seller_zero`, `seller_short`, `payout_zero`, `payout_upper`, `fund_zero`, `fund_upper`, `fund_short`, `token_zero`, `token_int` | `ok_other_chain`, `ok_zero_nonce_expiry`, `ok_source_chain_5`, `ok_fund_other` (`sellerFund` another address than `sellerKey`), `ok_int_top` (`chainId` 2^53 − 1, through `offer` and `order`) |
| §1 `zero`: every hex member the law tests for zero, with its index on a candidate's | `logic_zero`, `seller_zero`, `payout_zero`, `fund_zero`, `token_zero`, `judge_zero`, `judge_amanah_zero`, `judge_quote_zero`, `guild_pointer_zero`, `guild_amanah_zero`, `guild_quote_zero` | every accepted case; `guild_dup_first_of_two`'s second guild |
| `amount` dec256 nonzero (§1 dec256 at every edge) | `amount_zero`, `amount_leading_zero`, `amount_number`, `amount_too_big`, `amount_empty`, `amount_hex`, `amount_over_bound` (⌊(2^256 − 1)/10000⌋ + 1) | `ok_minimal` (1), `ok_max_amount_and_windows` (⌊(2^256 − 1)/10000⌋) |
| `deliveryWindow`, `confirmWindow`, `openWindow` u32 | `delivery_u32`, `delivery_string`, `confirm_u32`, `confirm_bool`, `openwindow_u32`, `openwindow_string` | `ok_minimal` (0), `ok_max_amount_and_windows` (2^32 − 1) |
| `buyerMayOpen` a bool | `buyeropen_int`, `buyeropen_string`, `buyeropen_null` | `ok_full` (true), `ok_minimal` (false) |
| §2.3 the goods pair, exactly one and of its form (`E_TERMS_GOODS`) | `goods_both`, `goods_neither`, `goods_md_int`, `goods_md_null`, `goodshash_short`, `goodshash_upper`, `goodshash_int` | `ok_full` (text), `ok_goods_hash`, `ok_goods_hash_zero`, `ok_empty_segments`, `ok_prose_unicode` |
| §2.3 the delivery pair (`E_TERMS_DELIVERY`) | `delivery_both`, `delivery_neither`, `delivery_md_array`, `deliveryhash_short` | `ok_full` (text), `ok_delivery_hash`, `ok_both_hashes` |
| `ladder` two to sixteen, the count before any rung | `ladder_one`, `ladder_17`, `ladder_object`, `ladder_one_bad_rung`, `ladder_17_bad_rung` | `ok_minimal` (2), `ok_16_rungs` |
| §2.5 rung table rows, each with its index | `rung_string`, `rung_missing`, `rung_closed`, `rung_buyer_over`, `rung_buyer_string`, `rung_seller_string`, `rung_bearer_3`, `rung_bearer_string`, `rung_note_int`, `rung_sum`, `rung_order_buyer_before_sum` | `ok_full` (a note), `ok_16_rungs` (every bearer) |
| §2.5 ladder-wide rules (`E_LADDER_FIRST`, `E_LADDER_LAST`, `E_LADDER_ORDER` with index) | `ladder_first`, `ladder_last`, `ladder_order`, `ladder_first_before_last`, `ladder_last_before_order`, `ladder_order_first_of_two`, `ladder_rung_before_first` | `ok_equal_rungs`, `ok_16_rungs` |
| `judges` an array; §2.5 judge table rows (`judge`, `amanahHash`, `quoteHash`, `source`), closed to four | `judges_object`, `judge_string`, `judge_missing`, `judge_missing_quotehash`, `judge_closed`, `judge_closed_quote`, `judge_zero`, `judge_is_seller`, `judge_amanah_zero`, `judge_amanah_short`, `judge_quote_zero`, `judge_quote_short`, `judge_quote_upper`, `judge_quote_int`, `judge_source_bad`, `judge_source_int`, `judge_source_other_chain`, `judge_source_upper` | `ok_full`, `ok_source_and_compliance`, `ok_source_chain_5`, `two_judges_same_quote_hash`, `ok_source_zero_address` and `ok_source_seller_address` (the two pointer rows differ: `source`'s address is untested) |
| no judge repeated, after every table | `judge_dup`, `judge_dup_after_tables`, `judge_dup_first_of_two` | `judge_and_guild_same_address` |
| `guilds` an array; §2.5 guild table rows (`pointer` as roster/1 §6.1, `amanahHash`, `quoteHash`) | `guilds_object`, `guild_string`, `guild_missing`, `guild_closed`, `guild_closed_quote`, `guild_pointer_zero`, `guild_pointer_no_colon`, `guild_pointer_prefix`, `guild_pointer_upper`, `guild_pointer_chain_big`, `guild_pointer_space`, `guild_pointer_empty_chain`, `guild_pointer_other_chain`, `guild_amanah_zero`, `guild_amanah_short`, `guild_pointer_seller`, `guild_quote_zero`, `guild_quote_short`, `guild_quote_null` | `ok_guild_only`, `ok_source_and_compliance` |
| no guild repeated; the two arrays not both empty | `guild_dup`, `guild_dup_after_tables`, `guild_dup_first_of_two`, `no_candidate`, `no_candidate_before_nonce` | `ok_guild_only` (judges empty), `ok_full` (guilds empty) |
| `nonce`, `expiry` ints; `compliance_md` an object where present | `nonce_string`, `expiry_null`, `compliance_string`, `compliance_null` | `ok_zero_nonce_expiry`, `ok_int_top` (both at 2^53 − 1), `ok_source_and_compliance` |
| §2.4's order, every adjacent pair of stages, and §2.5's row order inside each table | `order_v_before_chain` … `order_expiry_before_compliance`, `order_buyeropen_before_goods`, `goods_before_delivery`, `order_delivery_pair_before_ladder`, `order_ladder_rules_before_judges`, `order_judge_repeat_before_guilds`, `order_guild_element_before_repeat`, `order_guild_repeat_before_nonce`, `order_guilds_before_nonce`, `rung_order_*`, `judge_order_*`, `guild_order_*`, `*_order_element_before_element` | |

### §3 the quote (`quote`)

| boundary | fails | passes |
|---|---|---|
| 480 bytes | `len_479`, `len_481`, `len_512`, `len_0` | `ok_direct` |
| each word's rule in index order, `E_QUOTE_FIELD` with the eleven indices §3.2 can name | `w0_zero`, `w0_not_address`, `w1_over`, `w2_not_address`, `w3_not_address`, `w4_not_address`, `w5_over`, `w7_over`, `w11_over`, `w12_over`, `w13_three`, `w14_over`, `order_w*_before_w*` (every adjacent pair of faulting words); each bounded word with bits past its ABI type's width, read whole (§3.1): `w1_high_bits`, `w7_high_bits`, `w13_high_bits`, `w14_high_bits`; `w5_over`, `w11_over` and `w12_over` (2^32) carry the bit past `uint32` for the three words whose bound is their type's maximum | `ok_bounds` (every bound, `lapseDefault` 0), `ok_direct` (`lapseDefault` 2), `ok_lapse_half` (`lapseDefault` 1), `ok_reference` (`feeFixed` at 2^256 − 1), `ok_words_top` (`feeFixed`, `feeMin`, `feeMax` and `amountCap` each at 2^256 − 1) |
| `lapseDefault` 0, 1 or 2 | `w13_three`, `order_w12_before_w13`, `order_w13_before_w14` | `ok_bounds`, `ok_lapse_half`, `ok_direct` |
| the feed rule, after every word | `feed_token`, `feed_unit`, `order_field_before_feed` | `ok_reference`, `ok_one_feed` |
| HARNESS transport hex | `misuse_odd`, `misuse_digits`, `misuse_prefix`, `misuse_0X`, `misuse_none`, `misuse_two` | `ok_upper_transport` |

### §4.3, §4.4 the offer and the order (`offer`, `order`)

| boundary | fails | passes |
|---|---|---|
| `c` below the candidate count (`E_OFFER_CANDIDATE`), the document's fault first | `offer` 3, 9007199254740991, `NO_BLOCK` | `offer` 0, 1, 2; `offer_flags` (`buyerMayOpen` false, `openWindow` 0, a hashed segment) |
| `buyerKey` nonzero, not the seller's, not the candidate's judge or guild | `order` buyer zero, seller, judge, guild (`order_buyer_is_guild`) | guild candidate with the judge's address; the second judge's offer with the first judge's address; `order_buyer_is_fund` (the seller's fund is no excluded address) |
| §4.3 every member from the document, `sellerFund` apart from `sellerKey`; §4.4 the salt and the instance carry it; `prev`, zero or the instance of the stage before, is part of the salt and of every action's instance (`order_prev_nonzero`, `sign_order_prev`, `secp_prev_ok`, `secp_other_prev`, `act_confirm_prev`, `sign_action_prev`, `act_confirm_prev_ok`, `act_confirm_other_prev`), its form misuse on this surface (`E_ORDER_PREV`, with the other three derivation tokens' form halves; on each of the six commands once: an uppercase prefix, uppercase digits, no prefix, the empty string, nineteen and twenty-one bytes); §5.1 and §4.5 the key signs it, the fund nothing | `verify_offer_by_fund`, `act_refund_by_fund` (the fund's key: `E_SIG_SIGNER`) | `offer_fund_other`, `order_fund_other`, `verify_offer_fund_other`, `act_refund_seller_fund_other` (over `derivations_fund_other`) |
| `buyerPayout` nonzero; buyer before payout | `order` payout zero, both zero | payout = key, payout = seller |
| `openBy` an int (its form half is harness misuse) | `order` `01700086400`, `-5`, 2^53 | 0, 2^53 − 1, the default |
| HARNESS: `<c>`, `<buyerKey>`, `<buyerPayout>`, `<openBy>`, `<prev>` forms; an argument beyond the list | `offer` `01`, empty, `-1`, 2^53, short, beyond; `order` upper key, short payout, short, beyond | |

### §6 the fee (`fee <hexbytes> <amount>`)

| boundary | fails | passes |
|---|---|---|
| the quote first (§3.2's tokens) | `quote_len_first`, `quote_field_first`, `quote_feed_first` | |
| `priced` false where `feeUnit` nonzero, no sum formed there; the two bounds in order; `feeFixed` + ⌊A × feeBps / 10000⌋ above 2^256 − 1, decided before the bounds | `overflow`, `overflow_before_max` (a `feeMax` that would have lowered it) | `direct`, `reference_unit`, `unit_nonzero_no_sum`, `no_bounds`, `min`, `max`, `min_above_max`, `max_no_overflow`, `amount_zero`, `amount_max`, `min_top` (`feeMin` at 2^256 − 1), `max_top` (`feeMax` at 2^256 − 1 lowering nothing, `feeFixed` 2^255), `max_high_lowers` (`feeMax` 2^200 lowering `feeFixed` 2^201) |
| `premium` + `lf` above 2^256 − 1 after the bounds | `sum_overflow`, `sum_after_min` (the raise to `feeMin` makes it overflow), `min_top_sum` (`feeMin` at 2^256 − 1 with a premium of 1) | `sum_at_bound`, `min_top` |
| HARNESS `<amount>` dec256 | `misuse_leading_zero`, `misuse_too_big`, `misuse_exp`, `misuse_empty`, `misuse_hex`, `misuse_short`, `misuse_path` | |

### §4.5 actions (`action`, `sign-action`, `verify-action`)

| boundary | fails | passes |
|---|---|---|
| the six names, their argument counts (misuse), `E_ACTION_MANIFEST`, `E_ACTION_BPS` | `Bogus`, `Dispute 1`, `Cancel 0`, `Settle` short, `ClaimDelivery` short, bps not an int, `Confirm x`, no action; `ClaimDelivery` zero and short; `Settle 10001` | each of the six struct hashes; `Settle 10000 0`; `Cancel` under another `openBy` |
| `Settle`'s `expiry` an int | `Settle 5000 2^53`, `Settle 5000 01` | `Settle 5000 2^53 − 1` |
| the order's token before the argument tests; the derivation's before the order's | `action_order_before_args`, `verify_action_order_before_args`, `E_ORDER_BUYER`, `E_OFFER_CANDIDATE` | |
| the party rule after the argument tests and before any test of §5 | `act_*_party`, `act_cancel_seller_by_call_party`, `act_confirm_seller_by_call_party`, `act_claim_zero_buyer_manifest_first`, `act_settle_bps_before_party` | `act_*_ok` |
| the empty slot: `Settle` alone under either party; `E_SIG_FORM` for the other five under the party that admits each | `act_confirm_by_call`, `act_cancel_by_call`, `act_dispute_by_call`, `act_claim_by_call`, `act_refund_by_call`, `act_settle_zero_bytes` | `act_settle_seller_by_call`, `act_settle_buyer_by_call` |
| §5.3 one form for both parties: a string, an object refused alike | `act_claim_seller_object`, `act_settle_seller_object_form`, `act_settle_buyer_object_form`, `act_confirm_buyer_int` | `act_*_ok` |
| the digest binds the instance, the message, and the arguments; a later stage's instance is the one its `prev` derives (§4.4) | `act_confirm_other_openby`, `act_confirm_other_prev`, `act_claim_seller_wrong_digest`, `act_confirm_wrong_key` | `act_confirm_prev_ok` |

### §5 signatures (`sign-offer`, `verify-offer`, `sign-order`, `verify-order`)

| boundary | fails | passes |
|---|---|---|
| §5.1 hex65, `v`, range, high-s, recovery, signer, in galeed/1 §5.4's order | `0x1234`, uppercase, 66 bytes, `v` 0, 1, 26 and 29 (`v_zero`, `v_one`, `v_26`), zero `r` and `s`, `r` = n and `r` = 0 each beside an `s` in range (`r_n`, `r_zero_s_in_range`), `s` = 0 beside an `r` in range (`s_zero_r_in_range`), the high-s twin and `s` = (n − 1)/2 + 1 (`s_half_plus_one`), `r` = n − 1, `x` = 5, a recovery to the point at infinity (`verify_offer_infinity`), a real point's stranger, `sig_order_*` (each adjacent pair of `galeed/1` §5.4's tests, the range test failing once by `s` and once by `r`: `sig_order_r_zero_before_high_s`, `sig_order_r_n_before_high_s`) | the seller's signature, the guild candidate's, any key's signer; `s` = (n − 1)/2 passes the low-s test (`s_half`, to `E_SIG_SIGNER`) |
| §5.4 the empty string alone is the second form, after the document and derivation tests | `0x` as `<sig>`, `by_call_zero_bytes`, `NO_BLOCK` and candidate 2 with `""` | `""` on each candidate, `by_call`, `by_call_any_key` |
| §5.2 the buyer's string form, the value's form; §5.3 an object is no signature | `secp_form`, `secp_int`, `secp_null`, `secp_array`, `object_form`, `secp_high_s`, `secp_wrong_key`, `secp_other_payout`, `secp_other_openby` | `secp_ok` |
| §4.4 before §5 on `verify-order` | `secp_buyer_zero`, `secp_offer_fault`, `by_call_buyer_zero`, `by_call_offer_fault`, `by_call_payout_zero` | |
| HARNESS: keys and their spellings, argument counts | zero key, n, short key, a non-hex digit (`privkey_non_hex_digit`), short lists | key without `0x` in uppercase, with `0X` (`privkey_0X_prefix`), in mixed case (`privkey_mixed_case`), n − 1 on `sign-order` |

### Harness misuse

Every command that reads a path once with an unreadable one (`check`, `verify-order` and `gen_misuse`'s eight), every command once with an argument short and once with an argument beyond the list (`fee`, `sign-offer` and `sign-order` in `gen_misuse`, the rest in their sections), plus `ert` alone and an unknown command; a `<sig.json>` the reader refuses; `<party>` outside `buyer`/`seller`; and three where the document fails §2.4 and the misuse answers first (`offer` with `01`, `verify-order` with an unreadable `<sig.json>`, `verify-action` with a `<party>` outside the two).

### Clauses that state no boundary

§1's `base64url` states no boundary here: no predicate of this law reads it, and `eravon.link/1` §3.2 and §4.3 read it, the link corpus witnessing it on both sides. §1's derivation clauses (`word`, `DEPLOYER`, `initcode`, `runtime`, `proxyCodeHash`, `create2`, the EIP-712 restatement) and §4.2's type strings state no boundary of their own: they are measured by every offer, order and action digest and every instance address the corpus predicts, and `runtime` and `proxyCodeHash` are measured in the link corpus through `E_LISTING_CODEHASH`; no `ert` command answers them apart.
