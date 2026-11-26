# The standard's identity

Computed by `identity.sh` from the compiled creation code under the pinned
settings of `foundry.toml` (solc 0.8.28, evm_version cancun, optimizer 200,
via_ir, no metadata), standard s13. The judge declares `proxyCodeHash`; the
logic is deployed by whoever first uses it, through the public deployer
`0x4e59b44847b379578588920cA78FbF26c0B4956C` under the salt
`keccak256("eravon/1")`, and lands at `logic` on every chain the deployer
exists on; the chain's panel book under `keccak256("eravon/1/panel")`, the
verifier under `keccak256("eravon/1/verify")` and the board under
`keccak256("eravon/1/board")` the same way, each at its own address. Twelve
values (every table below the current one was computed under an earlier form
of the sources, with eleven values and a book per guild; the tables of
2026-09-24 and before under `eravon/3`, the name this standard carried until
that day).

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xc20db87ece6f54bbb9bdea43994aeeb7e8ff48dfaf24852626b16d2529f4aa14` |
| 1 | `logic` | `0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875` |
| 1 | `proxyCodeHash` | `0x9fcd87aa6d3f7b07234d4eba0cd8277506802498b41c377764ef05b3afb5583f` |
| 1 | `panelInitcodeHash` | `0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864` |
| 1 | `panel` | `0x4eD1d231cD27a84517F183f1455188713dbD06d1` |
| 1 | `panelCodeHash` | `0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8` |
| 1 | `verifierInitcodeHash` | `0x1b71b7e340dd6e1882635ae2b80b0dd4726e604608169e90a40eb091291b1596` |
| 1 | `verifier` | `0x86e73507BB79c888C968Fe7eF733688957af18aF` |
| 1 | `verifierCodeHash` | `0x2bbc633e66fd4b81e7660302716aec28dc978145093b7cb3cd2f399ad1c4991d` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

The verifier, `EravonVerifier`, is the pure and view half of `init`, of
`cancel`, of every draw and of every terminal: the terms guards, the quote,
the pricing, the two parties' consents at `init` and `settle` (`consents`: a
signature by the party's current key, or, on an empty slot, the caller being
that key or the logic's registry holding the digest under the party), the
buyer's and the seller's action signatures, the guild track's read of the
panel book at `init` (`panel`), the draw (`draw`, seating the series judge
first where no router is named), the salt and the address proof (`salt`,
`initSalt`), `cancel`'s four reads before the consent (`cancelCheck`), the
stage before's read and gate (`stageOf`, `stageGate`, standard s4a), and the
terminal split of standard s8 (`split`, which also answers `outcomeOf` and
`advanceOf`), deployed once per chain by whoever first needs it; the logic
derives its address and checks `verifierCodeHash` before every call. The
chain's panel book, `EravonPanelBook`, files each panel under the engaging
key's root on the succession book and the termsHash it is sent, with its
router and the router's code hash. The logic alone carries the three
registries, consent by call (`consent(party, digest)` from the party's
current key, `consented`, `Consented`), the seller's revocation of a terms
document (`revoke(seller, termsHash)` from the seller's current key,
`revoked`, `Revoked`) and the funding bonds (`acceptFunding`, `funds`,
`FundBound`), and the deposit face (`pull`, `Pulled`), which moves the
order's `sellerFund` allowance into the calling instance and nowhere else.
The board, `EravonBoard`, is the broadcast face (standard s7b): one
function, `post(key, document)`, one event, no storage. Runtime sizes under
the pinned compile: logic 24308 bytes, verifier 12906, book 1801, board 241;
EIP-170 allows 24576. Every measure is under forge 1.8.3; `identity.sh`
reads the artifacts under `out/` rather than embedding the contracts, since
a script that embedded every creation and runtime code passed 64 KiB and the
assembler's two-byte offsets.

Superseded before any deployment (the draw opened to any sender, 2026-10-27:
`redraw` tests no sender, its answer being the seed's and the panel's
whoever lands it, so that a party that is a company need not wait on its own
proposal window to draw; the verifier, the panel book and the board are
unchanged; no instance was ever opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xdff8e836023a8ad4e71e2c8698f425a3326b9a53ff53746e125b6e43c4f238e2` |
| 1 | `logic` | `0x9fa0B61E0107B9095C2D739f5cC5ACCF704DF1B7` |
| 1 | `proxyCodeHash` | `0xcf36b00725da7f65d61e665bd01197242bc73ace8febbd470cd9f3d16b6321f9` |
| 1 | `panelInitcodeHash` | `0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864` |
| 1 | `panel` | `0x4eD1d231cD27a84517F183f1455188713dbD06d1` |
| 1 | `panelCodeHash` | `0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8` |
| 1 | `verifierInitcodeHash` | `0x1b71b7e340dd6e1882635ae2b80b0dd4726e604608169e90a40eb091291b1596` |
| 1 | `verifier` | `0x86e73507BB79c888C968Fe7eF733688957af18aF` |
| 1 | `verifierCodeHash` | `0x2bbc633e66fd4b81e7660302716aec28dc978145093b7cb3cd2f399ad1c4991d` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the stages generation's first round,
2026-10-26: `stage()` answers a sixth word, the salt the instance proved its
address under, and the verifier admits as an instance only an account that
carries `proxyCodeHash` and is the address §1 derives from that salt (an
account whose storage no order wrote proves none); `stage()`'s judge is the
series judge, the judge who last heard a stage of the sale, handed on at
`init`; `cancel`'s address proof, `NotBuyer` and `BadTerms(4)` are read on
the verifier before the stage, and `init`'s address proof on the verifier
too; no instance was ever opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xc9b4893c93a43ed53a64f60acc2df3b579b83fd73bd74bf829e604380156ade5` |
| 1 | `logic` | `0xcB2d004727DDCf17808f1006527e7f26838bF155` |
| 1 | `proxyCodeHash` | `0x9ff635d17f9a4e05b912fd4d742d46f7228565c1440a7d3e5508f24d21e1eca0` |
| 1 | `panelInitcodeHash` | `0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864` |
| 1 | `panel` | `0x4eD1d231cD27a84517F183f1455188713dbD06d1` |
| 1 | `panelCodeHash` | `0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8` |
| 1 | `verifierInitcodeHash` | `0xabe4d300646215f5f6faf3e981bf69a63cf3681736d3e7d39493958d71e94de9` |
| 1 | `verifier` | `0x66B76f83dBEcDF3E4532157680F45C844C1Ba052` |
| 1 | `verifierCodeHash` | `0xe206a504a024a4ba9b69c6bfe88433ffa89d3d46aaaf85d30a2694e8184e6bbc` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the stages of 2026-10-26:
`OrderTerms.prev` in the type string and the salt, chaining a stage to the
stage before; `stage()` on every instance; `init` on a chained order after
the two consents requires the stage before to be an instance of this
standard with the same keys that ended leaving the buyer no share, `cancel`
one that ended leaving the buyer a share; the draw without a router seats
the stage before's judge first; no instance was ever opened on the table
below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xef3e463fd0de7d625f66e6bbcfd83221d9ed1fff751241ab91453c68b4b8cd1f` |
| 1 | `logic` | `0xcc5D2B12Dc6A262A28c53ffB6c81f813c674283c` |
| 1 | `proxyCodeHash` | `0x54ff3ece922e71c025fb43e0b581e60055b233c8643af82410092e13cc37cae0` |
| 1 | `panelInitcodeHash` | `0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864` |
| 1 | `panel` | `0x4eD1d231cD27a84517F183f1455188713dbD06d1` |
| 1 | `panelCodeHash` | `0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8` |
| 1 | `verifierInitcodeHash` | `0x646d1bb65d2490d247b8d412b9b419402a652bbf3e1003bde6de3acfe7d9f609` |
| 1 | `verifier` | `0x6134859aB097f2f62044a11FD137a37125257273` |
| 1 | `verifierCodeHash` | `0x85041dd6c397d26b07474b574bb034c01d3ecd80855407a98f89e1d6dbd7bba2` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded (the revision of 2026-10-24: every gate reads the current key
on the family's succession book at the constant the code names; a signed
form, `settle`, `sweep`, `redraw`, the expiry branch and `init` under
`buyerMayOpen` refuse every sender but a party's current key; `consent` and
`revoke` name the acting party; one panel book per chain, each record under
the guild's root with a router and its code hash, `init` freezing the root
and the latest record recorded before its block and unexpired at it, a
named router called by `staticcall` with exactly `routeGas()`; the venue
interface's `partyRoles`, `outcomeOf`, `advanceOf`, `successionBook` and
`routeGas`, and the seat's two credited fields; the quote of fifteen words
with no payout, a judge registering its own payout; `lapseDefault` 1 for an
even split at a lapse; P-256 removed). Never deployed on chain 1, where no
instance was opened on it; deployed at the same addresses on Sepolia
(11155111) for testing on 2026-10-28, with instances opened there, which
ERAVON's clients read as a past generation of that chain (ERAVON's report
of 2026-10-30):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xce856742d2e7d00a3b4fa13714cdbfbeff4b9109e5c3d0a674efded7bb8d5f16` |
| 1 | `logic` | `0x092d6f5B8078441958cD165919041e3AeEA01Bca` |
| 1 | `proxyCodeHash` | `0xabf13145f75f09139e0ff2b1ad9f379869ca80fa93451e22ea6dde467938ed47` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0x8d8f99b377394ce91adede1502e2a8019ec49195bfe271936d87bb4df5ea6630` |
| 1 | `verifier` | `0x1447a7B4D7150A3f4362788f9C490207EC713AFE` |
| 1 | `verifierCodeHash` | `0x488b4598d7515fefdd5a34791ed9b75c848de5ee78d233c4c4e17a8f73e94cd3` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the sellerFund revision of 2026-10-18: the
deposit is pulled from the order's `sellerFund`, the seller's key or an
address bound to the seller on the logic by `acceptFunding`, and returns
there; `Offer` gained `sellerFund` after `sellerPayout`, so both EIP-712
type strings changed; `Pulled` names the fund; `parties` has five
positions; guard 4 reads `sellerFund`; `FundUnbound` at the gate after the
consents; the terminal split of standard s8 moved to the verifier (`split`) for
room; no instance was ever opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x8870310a3427ca8702724b9e0a9fea4081210001646ef5f0c12bcf1038e45aaa` |
| 1 | `logic` | `0xB21e921128310852f81acF5072DcC5582F1adec8` |
| 1 | `proxyCodeHash` | `0x78ad9a4591bcafec164af23f2a77b464ebac9d5b4cbcc476eaed998be180dd72` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0x92c885f2d56fc0a42e863da9f1b13f2aa1025d275706ef08abc3fc041fdcba68` |
| 1 | `verifier` | `0x5cE086B1a3a9737c0429b3ba43eCe6e1E916fB61` |
| 1 | `verifierCodeHash` | `0xc46546575d0642136beafcb38780f31017fd3d160ddc4b5bebdf2ab4678c8c11` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the round-3 audits of 2026-10-17, four
seats with two findings of low severity: a seller's `refund` sent while
Claimed and landing behind the buyer's `dispute` executed as the surrender,
the seller losing P to the judge and its LF to the buyer by the
counterparty's ordering alone; `refund` is now refused while Disputed at or before T3's end, the
surrender row is gone, and `refundBySig` tests the signature before the
phase; the fuzz seat's R3-L1, a refund while Claimed under a seated seat
returning P to the seller, so that a refund and a payment off the instance
completed the trade with P stripped: such a refund now pays P to the judge,
as T2's lapse would; the verifier copies a feed's round to 160 bytes and a `decimals()`
to 32 and a push copies back no return data, so a long answer costs the
caller nothing beyond the allowance; no instance was ever opened on the
table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x593d751b9b0d5c9579e41e43ab89cbf71a487830e71501631b8ddac041cc9b73` |
| 1 | `logic` | `0xf841A1AAAADB38Efa191892d55fc326305337b41` |
| 1 | `proxyCodeHash` | `0x83b15d00b039652677c95a318d9aae4b1aac2ddcbe7a77ce9c9a8741f75b59e4` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0xbf1484bf0a26ce42124f5c82d81bb1477964ec7209a19e26a41987777f4cc35e` |
| 1 | `verifier` | `0x6EBc464ea028052E4A5F29ca4b259de3417c752E` |
| 1 | `verifierCodeHash` | `0x0d3584bbc068005e0fd8b106f42dbbc66de54c513529db740409763efd85ae73` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the round-2 audits of 2026-10-17, three
seats with no finding above medium: a guild draw whose seed the chain no
longer serves closes the case as Resigned with form 3, by `redraw` or by
`advance`, in place of re-arming, so that withholding a draw buys no second
seed; the block after an arming may always draw, so no `offerWindow` makes
a draw impossible, and the deadline governs from the second block on; a
settlement under a seated seat while the case is Claimed pays P to the
judge at any time, as T2's lapse would; the surrender sends the seller's
whole LF to the buyer whatever the last rung's `feeBearer`, and is the
seller's own call, `refundBySig` being refused while Disputed; `init` reads
both balances through `_balance`; every push of the standard's own, the
expiry branch's and `cancel`'s included, runs under the 1,000,000 gas
allowance; every read the pricing makes is handed 200,000 gas; no instance
was ever opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xcae0292a2163d980c1abb0ae87b43a925fd46fa812dcd016189ded702db5564a` |
| 1 | `logic` | `0x37C618e7411c298e9cBFDCb2A47950D5cf5FD7cE` |
| 1 | `proxyCodeHash` | `0xfa246fd44a81075ad4996c0484be80d9f2008377642dc9a3231954ad542473fb` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0xdf4e2ede6c3a1487fa53fdab374f9c1246f0f566c72d29d5aef13116a5ac03e0` |
| 1 | `verifier` | `0xeBE06Cd00c23d7f2331db747B351E7eBB54c8B83` |
| 1 | `verifierCodeHash` | `0xbab4ed4de06951bf7bdb168b4cf52d7e03f8396257e256849bded12f9811b3d7` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the fuzz seat's finding on the round-1
fold, found by the extended invariants before any audit of round 2: the
expiry branch and `cancel` credited the whole balance to the buyer's
payout, so a second run after a push the token refused counted what was
owed twice and the credit became unpayable; both now credit the balance
beyond what is owed; and a conversion with every absolute amount zero no
longer scales the price, so a scaled price past 2^256 − 1 is an overflow
only where something is converted; no instance was ever opened on the
table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x92b8b7f77fe3841dbc4da53ed25d1fe0b3353ccd6b6486e97db4601b832c779c` |
| 1 | `logic` | `0x7bAc8AC4837c6021Dbf54FFBa127B3eCeE4D61e8` |
| 1 | `proxyCodeHash` | `0x9fc722fc839b2779cbc6bc10fb94327397607eaaace50d9f1bdf3663f73eddc1` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0x537f39afb369851c2f57174f2e04e304447d4271d1aaf9192d6e811c55d7a85c` |
| 1 | `verifier` | `0xc8AEC9bcb3Eac22A62a08C988Cce4bfe82Cf2e67` |
| 1 | `verifierCodeHash` | `0x9ca42bf7bae1eb3a1d6723213c60141573fe1a66c54f06e400e0539459e51bcd` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the round-1 fold of the revision of
2026-10-17: the guild draw became two steps, `init` arming a draw seeded by
the hash of a block after the arming and `redraw` by anyone opening the
seat from the next block; the seller's refund during a dispute within T3
became a surrender at the last rung paying the judge; P at a settlement
only to an accepted seat whose window still ran; the amount bounded at
⌊(2^256 − 1)/10000⌋, the three ints at 2^53 − 1, the guild refused as a
party, the rung order decided after the first and last rung; the
conversion in full width and every overflow named `FeeOverflow`; feed
words read raw with a future `updatedAt` stale; passkey blobs and the
client data charset tested under `BadPasskey(0)`; the expiry branch and
`cancel` crediting and pushing best effort; `sweep` refusing a foreign
token that lowers the settlement balance; terminal pushes under a gas
allowance; `setPayout` refused on the logic; the seller's recovery and the
panel read moved to the verifier; no instance was ever opened on the table
below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x299f6e618256405cc0ef0002b815d6bbb441f2b9fc91e0b1e1c27a055cf97cf0` |
| 1 | `logic` | `0x560347d22FD6d9885488F2acC2C8B28ceEC4755e` |
| 1 | `proxyCodeHash` | `0x8e2c62aecb8b4981b17a26b19beceea20dcf953f5e118fab244e458aea23703d` |
| 1 | `panelInitcodeHash` | `0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e` |
| 1 | `panelCodeHash` | `0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd` |
| 1 | `verifierInitcodeHash` | `0x464b63fbb9ea2711b456d609bd6485d6de51c2d48c08482d11fef455ba9db2fc` |
| 1 | `verifier` | `0x23Cd9df85e088d7BD61CD7701b03103c07b79f59` |
| 1 | `verifierCodeHash` | `0x5ba709b344de7914804d76de564abb4b6af81d3a701268917ebf97e9428a853e` |
| 1 | `boardInitcodeHash` | `0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7` |
| 1 | `board` | `0x62D4040839c4d4b9480f0Fb60028b257acEf6420` |
| 1 | `boardCodeHash` | `0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353` |

Superseded before any deployment (the revision of 2026-10-17: the seller's
deposit became an ERC-20 allowance the logic pulls at `init` for P + lf,
`bond`/`bondFrom` and the bond's own money removed; `cancel` by the buyer
before any case, `revoke` by the seller on the logic, the expiry branch
widened to the order's `openBy` and a revoked document; `openWindow` and
`buyerMayOpen` in the signed offer and `openBy` in the signed order, so
both EIP-712 type strings changed; the caller rule where the buyer may not
open; a dispute names no rung and `Dispute()` carries none; a seat that
resigned while the case was Claimed leaves the case running on its clock;
P only to a seat that stood to the end; the board added; no instance was
ever opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x2d2c9e21023c9da14d21c983944562bf6c3c945a545a021baabea9656674ff82` |
| 1 | `logic` | `0xB9858EA7DED1885a91e056a9344ea27FfEbae5fE` |
| 1 | `proxyCodeHash` | `0x7ec2a0a1997505a8c6717ac4eb53fa783c65f0ee7d54c5be085f8eb99bbd740f` |
| 1 | `panelInitcodeHash` | `0xd5fc5e6d41680222c5772449fb887f0015f69810348fa2e26e25cb912898fcca` |
| 1 | `panelCodeHash` | `0x1e9588a891f0a69f33148506741029aeb36bc517f7e162e30030dcc5924f1153` |
| 1 | `verifierInitcodeHash` | `0xd64ad77f5b6899b2ecb8b975752f7534a03abcdf52806915a3e5bc197696c1b7` |
| 1 | `verifier` | `0x5DfF7de6B4610c7B48D76B9E50e6cdb31AbA0406` |
| 1 | `verifierCodeHash` | `0xd372eb9c2c88b4b3fdf4395aace96dcfeab5aa718261e91989e24d14066fdef6` |

Superseded before any deployment (the guild dues removed and the audit of
2026-09-26 folded, judge 2026-09-26: the book no longer carries
`setDues`/`duesOf` and takes its members strictly ascending; the instance no
longer freezes or splits dues and emits no `Dues`, debits a credit by what
left the instance, lets a foreign token leave to the buyer's payout, prices
nothing past `expiry`, refuses a late resignation on an accepted seat and
pays no P to a judge whose ruling window lapsed; no instance was ever
opened on the table below):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xa230ac8618fa4632ce0739df984ebcc8a9df22a34bd4bd9ffc276a6d29c0e287` |
| 1 | `logic` | `0x96e1708Df1A1821c635340B1d788dC976d8a2cBc` |
| 1 | `proxyCodeHash` | `0x780a4edea6fefdba7fea9d5d77df26adb25a046eafce9ec7417d792194e7765b` |
| 1 | `panelInitcodeHash` | `0xab48e0f1c48695671bd54bf64c86f501396ec7bf74718a1eda17114dba7dc35e` |
| 1 | `panelCodeHash` | `0xed86e1079dd2f44bdf05778515e083d63bd5bad1b66ce2cfeb0a1c058c6daa04` |
| 1 | `verifierInitcodeHash` | `0x2053f6c0e2103359aede6f4f30fe8263c2adb810688704fa50a38eb0947de764` |
| 1 | `verifier` | `0xdFaFC7A7bf57628296586f734B8761c3b13D93DD` |
| 1 | `verifierCodeHash` | `0xf299907953b16b74c79d09b64e5ddb282bce9c252081fc026b9fa80c2202fe20` |

Superseded before any deployment (the generation number: the judge ruled on
2026-09-24 that an unreleased standard carries no internal generation
number, so the salt `eravon/3` and the EIP-712 domain version "3" of the
consent-by-call standard became `eravon/1` and "1"; the sources are
otherwise the same and the sizes unchanged; no instance was ever opened on
it):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x13f45eb2ee05aa781a5497d9665a9cf518e854473f9dea2a21afbf9989fd4b54` |
| 1 | `logic` | `0xe51581df30a38176E3da32214ffa4713822605FB` |
| 1 | `proxyCodeHash` | `0x22b8b92372d4b06999e092591060e9ec4bef725f95aaf05a90841c6e3bfa662b` |
| 1 | `panelInitcodeHash` | `0xab48e0f1c48695671bd54bf64c86f501396ec7bf74718a1eda17114dba7dc35e` |
| 1 | `panelCodeHash` | `0xed86e1079dd2f44bdf05778515e083d63bd5bad1b66ce2cfeb0a1c058c6daa04` |
| 1 | `verifierInitcodeHash` | `0x2053f6c0e2103359aede6f4f30fe8263c2adb810688704fa50a38eb0947de764` |
| 1 | `verifier` | `0x33cbDB68656181cb195A302FCDCAE02F0FC34Ae4` |
| 1 | `verifierCodeHash` | `0xf299907953b16b74c79d09b64e5ddb282bce9c252081fc026b9fa80c2202fe20` |

Superseded before any deployment (consent by call, `eravon.terms/1` §5.4:
the registry on the logic, the empty signature slots of `init` and
`settle`, the two-party consent decision moved to the verifier; no
instance was ever opened on it):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0xa4b0e1fa4d62c8a08d6b67825aa809f2b5e59b755f4687032d9a5df909637a51` |
| 1 | `logic` | `0x60cD161C7F7EaD1050A7278eC04e99C5D2a772d6` |
| 1 | `proxyCodeHash` | `0x2765513cd3004c4857724d890774460206732ad63ee93234fb9500a302574375` |
| 1 | `panelInitcodeHash` | `0xab48e0f1c48695671bd54bf64c86f501396ec7bf74718a1eda17114dba7dc35e` |
| 1 | `panelCodeHash` | `0xed86e1079dd2f44bdf05778515e083d63bd5bad1b66ce2cfeb0a1c058c6daa04` |
| 1 | `verifierInitcodeHash` | `0xc0ca32857b8420bec56098944257fe4e805a55ad3b2481a1f67a7a6166d9de09` |
| 1 | `verifier` | `0xE10E1857B79bc460AeEAa6c422a038a98697Ef48` |
| 1 | `verifierCodeHash` | `0xb6be9a6f06dcb64be60eca3ccb71ecec06f1d2a8d72859b5ad0f3432798ccfd9` |



Superseded before any deployment (the logic of that table ran 26053 bytes
of runtime code, over EIP-170; the pure and view halves of `init` and the
buyer's signature moved to the verifier):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x23ac24c28bcb897953221e4a9f7351328562b154b023be847542a48f90931f7f` |
| 1 | `logic` | `0x25bd82D6C29fd8A7feeBBa70b2BF57Ce9D445BeB` |
| 1 | `proxyCodeHash` | `0xc88fd9a1e7284a561e2b98e8f5b1c5b77a983e22ed98b2bac400b248d72354a5` |
| 1 | `panelInitcodeHash` | `0xab48e0f1c48695671bd54bf64c86f501396ec7bf74718a1eda17114dba7dc35e` |
| 1 | `panelCodeHash` | `0xed86e1079dd2f44bdf05778515e083d63bd5bad1b66ce2cfeb0a1c058c6daa04` |

Superseded before any deployment (the guild dues were added to the book
and the terminal split, both since removed; no instance was ever opened on
it):

| chainId | key | value |
|---|---|---|
| 1 | `logicInitcodeHash` | `0x34c9097225a9c1ab22351514ede31007e2c092bb55d4c11821cbd3f4395bd812` |
| 1 | `logic` | `0xe99672C12282F8BEe35dC771eC1e03493D24931a` |
| 1 | `proxyCodeHash` | `0xe0b8f429f0506e53edfcd63bf1f7b11ba10adc8a268d08650fe02bf3d3567bb3` |
| 1 | `panelInitcodeHash` | `0x85149c96f9622ecb8fa0634101bee7fe718d6dcb52171086075b4f737450d0ce` |
| 1 | `panelCodeHash` | `0xd2ff3242e8fe785b386822b1f66f9d05f8986054096d496ca4d721ca7e40f855` |

The chain's panel book lands at `panel` above,
`keccak256(0xff ‖ deployer ‖ keccak256("eravon/1/panel") ‖ panelInitcodeHash)[12:]`,
and carries `panelCodeHash`; the verifier checks that hash before it reads
the book. A change to any byte of `src/` or any setting above is a new
standard with a new table; the old rows stay.
