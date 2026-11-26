# K1 · 读法选择账(crate `zikaron`)

`zikaron/1` 的法文本已退出基底;这个核由 `base/docs/zikaron-v2.md` §12.6 所载摘要 `0xbecf…32fc` 钉住,判据即 `base/zikaron-conformance/impl-py/`。旧法文本与判据若能读开,
判据束缚。本档逐条记本实现在散文歧义处所取的读法:输入形、两种读法、所取、理由,以及跑判据之后
的结局。判据与所取不同即改从判据,零呈裁(BRIEF「读法选择」)。

判据侧的验收在仓外临时副本进行(`/private/tmp/zk1-crit`,拷后逐文件 sha256 对 BASE-LOCK 核过),
`base/` 一字未动。

---

## N1 · `zk1 audit` 输入用 §3.5 的哪几测

**输入形**:`zk1 audit <path>`,HARNESS 说该文件「holds a JSON object (any RFC 8259 spelling)」。

**两种读法**:(甲)只测 §3.5 的 test 1 与 test 2(合法 UTF-8、一份 JSON-text,含 §3.1 的
整数域、深度 128、代理转义三条),此外照单全收;(乙)再加 test 3(每个对象的键两两不同),
两字符集(test 4、test 5)与往返同一(test 6)都不加。

**所取**:乙。**理由**:「任一 RFC 8259 拼法」放开的是拼法(空白、成员次序、转义选择),
放不开值域:一个键写两遍的对象在 §3.1 里不是一个值,后面所有按键取成员的读法都无从谈起。
两字符集是 §3.3 给条目字节定的,审计输入不是条目;往返同一更是接受测,与「任一拼法」直接相斥。

**结局**:先按甲写,`a140_input_duplicate_member` 判据出 NO_LABEL 而本实现出 COMPLETE;改从乙后
三种子零分歧。语料的第三方预测(`predicted_label`)与判据同侧。

## N2 · 锚记录与证据记录的表外成员

**输入形**:审计输入的 `anchors` 元素带第八个成员,`evidence` 元素带第五个成员。

**两种读法**:(甲)§9.4 对基底那句「an object bearing a member beyond its list is not a
`zikaron/1` basis」推及一切审计输入的对象,多一员即无标签;(乙)那句只管 §9.4 自己那三类基底
对象(法文明写「§6.10 reaches no object of this section」,section 指 §9.4),锚记录与证据记录
按 §8 与 §9.2 点名的成员读,多出来的是数据。

**所取**:乙。**理由**:§9.4 那句的主语是基底,且它自陈是为封住基底的形;§8 与 §9.2 定的是
记录「carries」哪些字段,从未说记录是闭元数的。闭元数在本法里只有信封(§4.1 明说「only the
envelope of §4.1 is arity-closed」)。

**结局**:判据同侧(语料 `a175_input_anchor_record_extra_member` 有标签,note 写「a record with
a member beyond its seven is read by its seven」)。三种子零分歧。

## N3 · 「两条在每个字段上都相同的记录是一条」里的「字段」

**输入形**:两条 `(chainId, tx, hash)` 相同的锚记录,其余七员相同,但其中一条多带一个表外成员。

**两种读法**:(甲)字段含表外成员,两条即分歧,无标签;(乙)字段只指 §9.2 点名的七员,
表外成员不参与,两条收成一条。

**所取**:乙。理由同 N2:表外成员是数据,不入任何谓词;若它能把一条记录劈成两条,它就成了
谓词的输入。**结局**:语料未直击此形(无双记录带表外成员的案),判据未反对;本条留作模糊战役
的靶,若判据另有读法,以判据为准。

## N4 · 审计输入五员的缺席与形

**输入形**:`root` 大写、`pile` 元素奇数个十六进制数字、`unavailable` 元素非 hex32、
`evidence.calldata` 奇数长、`pile` 整个缺席。

**读法**:§8 说审计是五输入的纯函数,§1 说 hex20/hex32 只有小写一种拼法。五输入缺一或形不合,
就没有一份 §9.4 的审计输入,出 `{"ok":false,"reason":"NO_LABEL"}`。**结局**:判据同侧
(a163-a173 六案)。三种子零分歧。

## N5 · 不可得集是集不是多重集

**输入形**:`unavailable` 数组里同一枚 hash 出现两次。

**读法**:§8 称它 unavailable **set**,§8.7 item 8 出「the unavailable set, sorted bytewise」。
本实现在收输入时即去重,报告里一枚只出现一次。**结局**:语料未直击;判据未反对。

## N6 · `zk1 sign` 不校验前像

**输入形**:`zk1 sign <privkey> <path> [domain]`,`<path>` 载六成员信封(或 §6.6 的 C)的正典
字节。**读法**:HARNESS 把「是正典字节」写成对调用者的交代,不是命令要复验的前提;`presig` 就是
`sha256(文件字节)`。本实现照此:不解析、不判正典,只取字节。**理由**:命令的用途是造语料与
两实现对签,加一道校验只会让「拿非正典字节对签」这类比较做不成。**结局**:判据同侧,39 个 sign
案三种子零分歧。

## N7 · §4.3 step 8 的两枚 token 各管一半

**输入形**:`prev` 是数字;`prev` 是 hex32 而 `seq` 为 0。

**读法**:step 8 先判形(null 或 hex32,否则 E_PREV),再判「null 当且仅当 seq 为 0」
(否则 E_PREV_SEQ),照「Where a step names more than one token, its tests are decided in the
order the tokens are written」。**结局**:判据同侧(E_PREV 23 案、E_PREV_SEQ 4 案)。

## N8 · 可选字段写成 `null`

**输入形**:`{"subject":null,"note_md":"n"}`。**读法**:§6 明写「an optional field written with
the value `null` is present with a form its row refuses」,故 E_BODY_FIELD,不当作缺席。
**结局**:判据同侧。

## N9 · 走链里「刚访过的那个 seq」跨组仍成立

**输入形**:同一 seq 上两条分叉双生子,前面还有缺口。

**读法**:§8.2 的「the `seq` visited just before it」是访问序上的前一条,与它是否同组无关;
故第二个双生子既不记缺口也不动 `expected` 与 `certain`。本实现按访问序维护 `prev_seq`。
**结局**:判据同侧(a048、a049 两案在 2^53 − 1 上直击)。

## N10 · 权柄只在一个位置的条目全部访毕后才换

**输入形**:一个位置上两条承继争位,其后位置的 author 只合其中一条的 `to`。

**读法**:§8.2 末段「After every entry at position `k` has been visited...」是按位置换,不是按条目
换;同位置内各条仍按同一枚 `auth` 评权。**结局**:判据同侧(a024-a027、a030、a031 六案)。

## N11 · §9.5 的对齐搜寻

**输入形**:content 落在 calldata 的偏移 0、4、32、36、8。**读法**:偏移对 32 取余为 0 或 4,
且 32 字节整体落在 calldata 内;逐偏移搜,命中即证。偏移 8 不合对齐,即使字节在那里也未证。
**结局**:判据同侧(a104-a115 六案两色)。

## N12 · 第三方密码学的边

**读法**:secp256k1(k256)、sha2、sha3 只在 `src/cryptox.rs` 出现;曲线常数 n 与 (n−1)/2 的
两测(§5.4 的范围与低 s)也住在那里,免得曲线细节漏到 `entry.rs`。别处只调函数。RFC 6979 与
低 s 由 k256 的 `sign_prehash_recoverable` 承担,是否与判据同字节由 39 个 sign 案作证。
**结局**:三种子零分歧,签名逐字节一致。

## N13 · 量测面的字节串编码收大小写,法的对象不收

**输入形**:HARNESS 审计输入里 `pile[i]` 与 `evidence[i].calldata` 的 `0x…` 串混进一个大写
十六进制字;以及 `root`、`anchors[i].tx / sender / hash`、`unavailable[i]`、
`evidence[i].tx / sender`、基底的 `registries` / `senders` / `bareTx[i].tx` 各混一个大写字。

**两种读法**:(甲)一切 `0x…` 都是法 §1 的拼法,只认小写,任一处大写即不是审计输入;
(乙)分两层:法的对象(hex20 / hex32 / hex65,§1 明写小写)只认小写,而 HARNESS 的
`pile` 与 `calldata` 是量测面之间的**传输编码**,两案皆认、解出同一串字节。

**所取**:乙。**理由**:§1 定的是法的对象怎么写;`pile` 与 `calldata` 不是法的对象,
是 HARNESS 为把「一条字节串」搬进一份 JSON 而定的编码(法只说它们是 byte string)。
两层混同的代价是:一个大写字就把整份输入判成非审计输入,而那份输入的语义一点没变。

**怎么定的界**:逐位置对判据跑。十三个位置各造一枚只改一个字的变体,问判据答什么:
`pile` 与 `calldata` 两处判据照答报告(且与全小写的报告逐字节相同,大小写混写亦然),
其余十一处一律 `NO_LABEL`。`0X` 大写前缀在两处也不认。命令行上的三处另有其形:
`zk1 sign` / `zkk sign` 的私钥是**标量**(可带 `+`、可带 `0x` 或 `0X`、数字间可有下划线、
位数不论、大小写不论;值须在 1 到 n − 1,否则量具误用);`zkk depth` 的作品摘要是法的对象
(严格 hex32 小写,别的拼法退 2);`--now` 是 §3.1 的 int(只许十进制数字,前导零可有,
至多 2^53 − 1)。四处共 71 枚差分探针,零不符。

**结局**:利未的收敛闸核侧十万案模糊出的 13 例分歧,单一根因即此;改读法后十三份留证输入
逐份两侧同答。判据同侧,三种子重跑零分歧。

**留着没收的一处(记名不修)**:判据那侧的私钥读法是 Python `int(s, 16)`,它连 Unicode 空白
(如 U+00A0)与 Unicode 十进制数字(如 `\u{0661}\u{0662}`)一并收;本实现只收 ASCII 空白与
ASCII 十六进制数字。要抹平这一格,得在产品里带一张 Unicode 数字值表去模仿判据的一处实现细节,
而法对此一字未言,语料与两侧战役都到不了这条路径(私钥来自 meta.json,由比对器自己读了再
原样交给两侧)。故记名不修,随账申报。ASCII 空白那一格是白拿的,已收。

**落点**:`hexfmt` 分两族函数——`is_hex20 / is_hex32 / is_hex65` 只认小写(法的对象),
`decode / is_bytes` 两案皆认(传输编码),`scalar32` 认标量的诸形。别处不再自判大小写。


## N14 · 基底换版(Court bdaaea0):按新文本改动的句子,逐条

法全文重做,判据核摘要 `0xbecfb6f0…32fc`。下表每行一句:法条号、旧读法(本实现原来怎么做)、
新读法(新文本怎么说)、落点。判据同侧与否由六种子的 compare 读数作证(交账附)。

| # | 法条 | 旧读法 | 新读法 | 落点 |
|---|---|---|---|---|
| 1 | HARNESS `audit` 读者 | test 1 到 3,代理转义一律 E_JSON(与条目同) | RFC 8259 加五条限制:代理**对**读成一个标量值,孤代理项拒;条目(§3.3)仍拒任何代理转义 | `json::parse_tests_1_3` 走 `parse_with(b, true)`,`Parser.pairs` |
| 2 | §8.7 报告 | 十一项、十二成员 | 十五项、十六成员:新增 `anchored`(item 5)、`unknown_type`(9)、`malformed`(10)、`discarded`(14) | `audit_full` 报告装配,`tokens::Key` 十六员 |
| 3 | §8.7 item 4 | MISSING 行的 anchors 元素 `{chainId, tx}` | `{chainId, blockNumber, tx, sender}`,按 (chainId, blockNumber, tx) 排 | `anchor_row(a, false, false, false)` |
| 4 | §8.7 item 5 | 无 | ANCHORED:entry_id 在 counted 集的账本条目各一行,元素 `{chainId, blockNumber, blockTimestamp, tx, sender}` | `anchored_rows` |
| 5 | §8.7 item 9 | 无 | UNKNOWN_TYPE `{seq, entryType, entry_id}`,类型不在七员者,按 (seq, entryType, entry_id) | `EntryType::Other` 与 `unknown_rows` |
| 6 | §8.7 item 10 | 不过 §4.3 的字节只算 entry_id | MALFORMED `{entry_id, token}`,字节全等者收成一条,按 entry_id 排 | `audit_full` 的 `malformed` 收集 |
| 7 | §8.7 item 12、13 | UNPROVEN / VOID 行 `{hash, chainId, blockNumber, tx}` | 加 `sender` | `anchor_row(a, true, false, false)` |
| 8 | §8.7 item 14 | 世系修剪弃掉的记录无声消失 | DISCARDED 带七成员,按 (sender, chainId, blockNumber, tx, hash) | `partition` 出 `discarded` |
| 9 | §8.7 item 15 规则 2 | UNPROVEN 表非空即 UNAVAILABLE | 不可得集非空,**或** UNPROVEN 表里有一条记录其 hash 既不是堆中任一字节串的 entry_id、又不是未弃 counted 记录的 hash | `label_of` 第三参 `unproven_unbacked` |
| 10 | §9.4 去重键 | 锚记录按 `(chainId, tx, hash)` 收一 | 按 `(chainId, blockNumber, tx, hash)`;同键而 blockTimestamp / sender / verdict 有分歧即无标签 | `validate_inputs` 的 `seen_anchor` |
| 11 | §9.4 可达律 | 无 | 每条锚记录须在基底所及之内:`(chainId, tx)` 是某 bareTx 对,或其链的某窗含其 blockNumber 且 senders 持其 sender;否则无标签。证据记录的 chainId 须有 adoptionChains 对象点名 | `Basis::reaches`,`validate_inputs` |
| 12 | §9.4 基底之序 | 三表只查形与唯一 | `chains` 按 (chainId, fromBlock) 严格递增、同链不重叠、紧邻两窗须在 registries 或 senders 上有别;`bareTx` 按 (chainId, tx) 严格递增;`adoptionChains` 按 chainId 严格递增;`registries` / `senders` 字节序严格递增 | `validate_basis` |
| 13 | §9.2 / HARNESS `evidence.calldata` | 两案皆认 | `0x` 加偶数个**小写**数字 | `hexfmt::is_lower_bytes` |
| 14 | HARNESS `pile` | `0x` 两案皆认 | 前缀两字节须小写 `0x`,数字两案皆认(`0X` 在形外) | `hexfmt::is_bytes`(原已如此,现由 HARNESS 明写) |
| 15 | §8.2 承继换席 | 位置 k 上 entry_id 最小的承继的 `to` 成为其后权柄 | 只在 author 等于本位 auth 的承继里取 entry_id 最小者;没有即不动 | `walk` 的 `succs` 过滤 |
| 16 | HARNESS `sign` 的前像 | 按文件原字节取 sha256,不解析 | 任一 RFC 8259 拼法读入、重正典后按原值签,一个成员也不去;过不了 test 1 到 5、或值不是对象,皆量具误用 | `bin/zk1.rs` `sign` |
| 17 | HARNESS `sign` 的私钥 | 标量的诸形(可 `+`、可下划线、位数不论) | 恰六十四位十六进制数字,两案皆认,前可带 `0x` / `0X`;别的皆量具误用(N13 的这一段作废) | `hexfmt::scalar32` |
| 18 | HARNESS `sign` 的域 | 缺省 `zikaron/1`,给了即按原样 | 给了按原样签,空串也签;每字节低于 0x80 且无换行,否则量具误用 | `bin/zk1.rs` `domain_arg` |
| 19 | HARNESS 参数单 | 多出的参数忽略 | 超出命令自己那张单的参数即量具误用;退出码只有 0 与 2 | `bin/zk1.rs` `arity` |
| 20 | §10 / §6.9 | 条目类型只在 `check_body` 里按字面分派 | 七员认得的与表外的一员分开读(`EntryType`),报告的 UNKNOWN_TYPE 由它出 | `Entry.kind` |

未改而重核过的句子(新文本明写、本实现原已如此):§8.1 不可得集的两条弃律;§7.4 两个不动点
(整集世系遍历堆中一切过 §4.3 的承继,前缀世系只遍历账本承继且 seq < k);§9.5 得证三条件;
§3.5 test 2 的并列律(`+` / `.` 起头的值位名 E_NUMBER,`0x10` 名 E_JSON,`-Infinity` 名 E_NUMBER);
§5.7 低 s 取反翻 v(k256 的 `sign_prehash_recoverable` 即此)。


## N15 · 命令行参数不是 UTF-8 时(code-review 修法所载的读法)

**输入形**:`zk1 check <path>` 的 `<path>` 带一个 0xFF 字节(操作系统交来的路径不是 UTF-8)。

**两种读法**:(甲)`std::env::args()`,非 UTF-8 参数 panic,退 101、无输出;
(乙)`args_os()`,路径按操作系统交来的字节原样递给文件系统,文本参数(命令字、私钥、域)
不是 UTF-8 即量具误用退 2。

**所取**:乙。**理由**:HARNESS 两句:「No command exits with any code but 0 and 2」与
「A `<path>` argument is handed to the file system as the operating system delivered it,
whatever its bytes」。甲在第一句上就红。**结局**:三条实起探针(check 路径、kit-verify 目录、
grant-check 的 `--audit` 路径各带 0xFF)皆退 2、stdout 空。落点 `bin/zk1.rs` 与 `bin/zkk.rs`
的 `read(&OsStr)` 与 `text()`。

同一批 code-review 的另一处修法:§9.5 的对齐搜寻只走能命中的偏移(每 32 字节两处),
行为不变;逐字节走是收养元素数乘 calldata 长的挂起面,16 MiB 的合法输入可喂到二十余秒。


## N16 · 拒因表按路分子表(接缝批 S15)

**读法**:法 §10 的二十六员是一张表,但产品里吐它们的建立者各只吐一段:`json` 的读者只吐
§3.5 的八枚,`verify_signature` 只吐 §5.4 / §5.5 的六枚,十三步的 `check` 才吐全表。故立两张
子表 `CanonToken`(8)与 `SigToken`(6),各经 `From` 升为同名的 §10 token,字节不变;`check`
的签名仍是全表 `Token`(二十六员的承载)。kit 法 §3.1 照字借 §5 的六枚,§2.1 照字借 §3 的八枚,
两张子表正是它借的那两段,kit 的 `DocToken` 以 `Canon(_)` / `Sig(_)` 收它们。
**结局**:HARNESS 输出逐字节不变(compare 三种子作证)。

---

## 疑基底有病

无。三种子零分歧,未出现「本席判读法有据而判据背离」的形。
