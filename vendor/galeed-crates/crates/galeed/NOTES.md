# K1 · NOTES(读法选择逐条;与判据分歧处判据为准,不呈裁)

r1 批二(§3 到 §6、`gl1 check|canon|sign`)。每条写:法文本何处、读成什么、为什么这样读。

## §3 正典形(src/json.rs)

1. **第 1 道 UTF-8 用 std 的判定**:`std::str::from_utf8` 恰是 RFC 3629(最短形、拒代理、拒 > U+10FFFF),整串先判,再读文法。
2. **第 2 道一趟读停在首错**:法说「一趟从左到右的读,停在它的第一个错处,即实现此律」。并列错按 `E_DEPTH` > `E_NUMBER` > 代理转义 > 文法错只在同一触发字节上才相干;数形值位先切最大连串再判,故 `+`、`.` 开头必 `E_NUMBER`,`0x10` 是 `0` 过后 `x` 文法错。
3. **数形连串**:`[0-9+-.eE]` 的最大连串;整数形是 `0` 或非零开头的纯数字且 ≤ 2^53 − 1;连串为空(不可能,首字节已在集内)按 `E_NUMBER`。
4. **代理转义配对也拒**:§3.3「whether or not a second such escape follows it」,`😀` 是 `E_JSON`。(HARNESS.md 的 audit 输入读者另有一律「配对读作一个标量」,那是批三的另一只读者,不共用这一处。)
5. **字串里的原生 0x00–0x1F 是文法错**(§3.3 末段),不论散文与否;散文的许可只及解码后的标量值。
6. **第 3、4、5 道各整体决完**:重名遍历每层对象与其写出的成员序(含重复);键/值字符集遍历整棵值树。第 3 道范围含散文子树内的键(键始终归键律)。
7. **散文子树**:解码后的键恰是 `_md` 或以 `_md` 结尾即开;子树内任意深度字串值皆散文,含数组内;子树内的键仍是骨架非空。
8. **正典写法**:`/` 不转义;U+007F 与 C1 原字节;`\u00xx` 小写;成员按解码键的 UTF-8 字节序稳定排序(第 4 道已限键在 ASCII,与 RFC 8785 的 UTF-16 序在此无差)。
9. **十六 MiB 界之上**:答,不拒(HARNESS.md 许两者;整档读进内存即判)。

## §4 信封与十三步(src/envelope.rs)

10. **第 3 步先缺后多**:§4.1 两测,缺任一即 `E_ENVELOPE_MISSING`,否则有第八键即 `E_ENVELOPE_CLOSED`;成员数不决。
11. **第 4 步 spec 非字串同 token**:`E_SPEC` 一枚盖非字串与不等两形。
12. **第 5 步 token**:非空且不含 U+0020;其余字符集第 1 道已决(根成员不在散文子树)。
13. **第 8 步两 token 按写序**:先 `E_PREV`(形)后 `E_PREV_SEQ`(与 seq 的关系)。
14. **第 11 步**:`entryType` 逐字节等于 `genesis` iff `seq == 0`。
15. **entry_id**:`sha256(b)` 于原字节,与正典字节同(第 6 道已保证相等)。

## §5 签名(src/sign.rs、src/cryptox.rs)

16. **B6**:只去顶层 `sig`,body 内同名成员不动;从已解析的值重写正典字节(输入已过第 6 道,与切字节等价)。
17. **域字面只住一处**:`galeed::LAW`;`sign` 命令的 `[domain]` 按 HARNESS.md 收任意 < 0x80 无换行的字串(空串也收),不查是否登记(登记检查归 K5 检查器与签名 API 放行面,不在 `gl1` 夹具面)。
18. **RFC 6979 自写在 cryptox**:HMAC-SHA256;消息输入是 32 字节 digest 不再散列;`bits2octets` 按 RFC 取 digest 模 n;候选 k 出界、`R.x ≥ n`(r 不是 x 本身)、r = 0、s = 0 都按 RFC 的拒绝路再抽(K = HMAC(V‖0x00),V = HMAC(V));不用 k256 的 ecdsa 面,只用其曲线算术(`arithmetic` feature),法文本的每条细则在代码里逐句对应。
19. **低 s**:`s > (n − 1)/2` 即 `n − s` 且 `v = 55 − v`;`v` 由 R.y 奇偶取 27/28。判高低用 k256 的 `IsHigh`(同一常数的正主是曲线本身)。
20. **恢复**:只试 x = r(不试 r + n),y 奇偶按 i;`P = r⁻¹(sR − zG)`,z 是 digest 模 n;无点或无穷远即 `E_SIG_RECOVER`。地址取未压缩 64 字节坐标的 keccak 末 20 字节。
21. **第 13 步次序**:`E_SIG_V`、`E_SIG_RANGE`(r、s 任一出 [1, n−1])、`E_SIG_HIGH_S`、`E_SIG_RECOVER`、`E_SIG_SIGNER`。
22. **privkey 参数**:64 位十六进制任一大小写,`0x`/`0X` 可有可无;标量出 [1, n−1] 即误用(退出码 2)。

## §6 body 表(src/body.rs)

23. **在场即键在**:选填写 `null` 是「在场且形不合」,`E_BODY_FIELD`。
24. **形的读法**:`prose string` 只问是字串;`skeleton string` 问字串且非空处非空;`token` 问字串、非空、无 U+0020;`int` 问是整数(数形出界第 1 道已决);`bool` 问两字面;hex 按 §1 形。
25. **subjectRef**:对象且 `kind` 是 token;已知 `evm`/`hash` 强制其表(按字节匹配,大小写异即未知);未知 kind 不再测。`hash` 的 `note_md` 选填散文。
26. **resignation 唯一关系**:`clause` 在场 iff `category == unlawful-subject`;`category` 闭集两值逐字节。
27. **未知 entryType**:除第 9 步「是对象」外不测(§6.8);`galeed.dispatch/1` 在此与任何未知类型同待。
28. **数组字段**:零元素满足;元素各按其表;表外成员一律许可不读(§6.9)。

## HARNESS.md 出面(src/bin/gl1.rs)

29. **误用**:参数数不合、读不到路径、privkey/domain 形不合、待签档过不了 §3.5 前五道或非对象,一律退出码 2、stdout 空、stderr 一行;`audit` 本批亦误用。
30. **答复对象由本 crate 的正典器写出**:成员序由正典器排,不手拼。
31. **路径字节原样**:`OsString` 直接交 `fs::read`,不解码。

---

r1 批三(§6.7、§7 到 §9.6、`gl1 audit`、`gl1 kit`)。

## audit 输入读者(src/json.rs `read_open`,src/audit.rs `read_input`)

32. **两律一台读机**:HARNESS.md 的五限制读者与 §3.5 的条目读者共用一台 RFC 8259 读机,差一个开关:条目读者见任何代理转义即 `E_JSON`;audit 读者把高低配对读作一个标量、孤立代理拒。字符集(§3.3)不及 audit 输入;第 6 道(正典同一)不问。读不成即 NO_LABEL。
33. **字串里的原生控制字节**在两律里都是文法错(RFC 8259 `unescaped` 不收);转义的控制字符在 audit 输入里许可(a443)。
34. **良构一律 NO_LABEL,不分先后**:根非对象、五必填缺或形不合、pile 元素非 `0x`+偶数位、记录七成员缺或形不合、四同三异、基准的形/序/arity/唯一性、记录不可及,任一即 `{"ok":false,"reason":"NO_LABEL"}`;只有读不到路径与坏参数是退出码 2。表外成员(输入对象、记录、基准之外)一律不读。
35. **pile 元素**:`0x` 小写前缀,数字任一大小写;`0x` 是空字节串(其 entry_id 照算,§4.3 败于 `E_JSON`)。
36. **记录去重**:以 (chainId, blockNumber, tx, hash) 分组,组内七成员逐字比,异即 NO_LABEL;表外成员不参与比。
37. **可及**:bareTx 点名 (chainId, tx),或某窗口同链且范围含块且 senders 含 sender;`registries` 不读(§9.4 原句),它收了类项之后仍不读(r6 批一;类项命中哪个发出者归 A3 现核 `extcodehash`,K1 只判基准的形与序并收记录)。良构在 §8.1 修剪之前判,世系会丢弃的记录照样能坏输入。

## §7、§8(src/audit.rs)

38. **世系**:root 种子;每条过 §4.3 的 succession(不论其 judge 是否在职)只要 judge 已在集内就把 `to` 加进来;最小不动点循环到不再增长。seq 0 不可能是 succession(§4.2),不另判。
39. **走链的 gap 判**:`seq != expected && seq != last`;首访无 last。gap 后 `expected = actual + 1`,`actual` 为 2^53 − 1 时不加;非 gap 且 `seq == expected` 同律加一。
40. **PREV_MISMATCH** 只在 `seq − 1` 位置有账本条目时判;prev 落在被排除条目上算不匹配(a334)。
41. **移席**:一个位置的条目全访毕后,judge 等于当前 auth 的 succession 里 entry_id 字节序最小者的 `to` 成新 auth;无则不变。AUTHORITY_MISMATCH 用访到时的 auth 与 certain。
42. **等价两方**:同 seq 与同非空 prev 两个理由合成一个无序对集合,每对一条;seq 取小。
43. **不可得集修剪**:留下的恰是「是某条未丢弃 counted 记录的 hash」且「不是 pile 里任何字节串的 entry_id」者。
44. **各表序**:findings 按 (position, name, entry_id, second);anchored / unanchored / missing / unavailable / malformed 按 hex 字节序;excluded (seq, id);unknown_type (seq, entryType, id);unproven / void (hash, chainId, blockNumber, tx);discarded (sender, chainId, blockNumber, tx, hash);行内 anchors (chainId, blockNumber, tx)。int 数值序、字串字节序。
45. **标签第二条**:UNPROVEN 记录的 hash 既不是 pile 里任何字节串的 entry_id、又不是未丢弃 counted 记录的 hash,才移标签。
46. **报告的 basis** 就是输入里那个值,经正典器重写;`entries` 是账本条目数(逐字节同者已合一)。

## §9.6 取证包(src/kit.rs、src/rlp.rs)

47. **[X] 类,判据外**:corpus 与冻结核不覆盖它;本批以自建 MPT 树(测试内)两形各一见 counted、一见 VOID、一见 UNPROVEN、两见「什么也不显示」;真链两色留给 g2 的 anvil 场务(A2 取证包捕获)。
48. **包的传输形**是本席定的(档头注释);两块哈希答出给调用者自己绑信任,本函数不持任何块哈希表。
49. **发送者**:legacy 交易 v < 35 即「签名不点名链」,不是锚;0x01 到 0x04 四封套按各自字段位取 chainId、to、data;签名 r、s 要在范围且 s 低(EIP-2),恢复经 cryptox。交易的 chainId 不等于包点名的链即不显示。
50. **收据**:状态字段是 32 字节(前 Byzantium 根)即「无状态字段」不是锚;状态非 1(回滚)不是锚;typed 收据去掉首字节再读。
51. **registry 形**:emitter 由包给(收据日志所载的 address),日志恰三 topic、data 空、topic 0 = keccak("Anchored(address,bytes32)")、topic 1 = 12 零字节加发送者、topic 2 = 所称 hash,且 hash 在 calldata 里以 32 的倍数或 4 加 32 的倍数为起点整个出现;创建合约的交易(to 空)不是锚。
52. **bare 形**:to = 发送者,calldata 是正 32 倍数,所称 hash 是其中一词。
53. **两界**:0 号块的前界是创世前,算已问无码、不收前证明;其余缺哪一界的证明即 UNPROVEN;两界皆问而任一有码即 VOID;账户不在树(排除证明)即无码;codeHash 等于空串 keccak 即无码。证明不成立(根不对、节点缺、形坏)即什么也不显示。
54. **MPT 证明**:哈希引用在节点列里按 keccak 找;短于 32 字节的内嵌节点按父节点里的原字节切出;路径走到分支的空槽、叶不匹配、扩展不前缀即排除证明成立。
55. **`gl1 kit`** 是 HARNESS.md 四命令之外的第五个动词,compare.py 不碰它;读不成的包答 `{ok:false, reason:"kit"}`,读不到路径才退出码 2。

---

r1 批四(K5 检查器与签名放行面;`literals.rs`、`sign.rs`)。

56. **登记簿嵌入产品**:`FAMILY-LITERALS.md` 以 `include_str!` 编进 `galeed`,签名面的放行表从它现读(一名一庙);盘上登记簿改了而未重编,检查器规则⑥即红(嵌入表对盘上表)。
57. **两个签名面**:`sign::sign`(哈尼斯面,`gl1 sign` 用,照 HARNESS.md 收任意 < 0x80 无换行字面)与 `sign::sign_registered`(产品面,只放行登记五域,`UnregisteredDomain` 拒)。产品里 H2/V 组一律经后者;前者只给夹具。
58. **闭表的现读**:两法的域闭表从标记段落(`**5.6 Domain.**`、`**[D] 3.2 Domains.**`)到下一个空行之间取反引号内以 `galeed` 开头且含 `/` 的字面;zikaron 互异对照取两法文本全文里以 `zikaron` 开头且含 `/` 的一切反引号字面(过取无害:任何交集都该红),加登记簿 `foreign.zikaron.literals` 的离线下限。
59. **合约名现读**:源里以 `function ` / `event ` 起头的行取到 `(` 前的名;`amanahHash` 参数名不在此集,按 abi.kind 只比 function 与 event。
60. **检查器出面**:`literals check <registry> <docs-dir> <zikaron-dir> <contract.sol>`(四件;r6 批一一度加过场所接口源第五件,同批按炽天使裁定退回);答 `{ok, rules:[{rule, ok, detail}]}` 退出码 0(不过也是答),读不到路径、坏参数退出码 2;zikaron 目录取其中一切 `.md`。
61. **K5 与 K1 的边**:检查器与放行面住 `galeed` crate(K1 的 crate)而非独立 crate:放行表要由签名面自己读,拆出去就成两处读同一份登记簿。K5 在功能册上仍是自己的节点,工序单指认 `literals.rs` 与 `sign.rs` 两处。

62. **RLP 嵌套上限 64**(code-review 一审):无界递归对着外来字节即栈溢出;以太坊结构不过几层。长形长度用 checked_add,溢出即形坏。
63. **账户证明三态**(code-review 一审):键缺席是「没问过」(UNPROVEN);在场而读不成或不成立是「什么也不显示」。MPT 子引用同律:空槽与坏引用是两件事。
64. **EIP-7702(0x04)十三字段**(code-review 一审):批三误抄 0x02 的十二;签名原像盖前十字段。
65. **法文本字面的现读不靠标记**(code-review 一审):按字面字符连串取,奇数反引号与围栏不再使后文漏收;zikaron 目录一份法文本都没读到即规则③红,不许只靠留档下限静默成绿。
66. **钥的派生与备份只住 `cryptox`**(r5 批二,法外):BIP-39 英文词表嵌在 `src/bip39-english.txt`(与 BIP-39 规范仓的 `english.txt` 逐字节同,sha256 `0x2f5eed53…dbda`);助记词按 ASCII 空白切词、逐字节比词表;BIP-39 口令只收 ASCII(NFKD 对 ASCII 是恒等,非 ASCII 答无种子,不自己做 Unicode 归一);BIP-32 的指纹要 RIPEMD160(依赖 `ripemd`),`xprv` 自写 base58check;路径每段无前导零、在 2^31 以下,`'` 表硬化。
67. **keystore v3 的读写**(r5 批二):读收 `crypto` 小写成员(geth 旧档的 `Crypto` 不收)、`cipher` 只 `aes-128-ctr`、`kdf` 收 `scrypt` 与 `pbkdf2`(`prf` 只 `hmac-sha256`)、`dklen` 只 32;档里写了 `address` 即核;写只写 scrypt(`n` 2^18、`r` 8、`p` 1)与随机盐、IV、UUID v4。scrypt 用 RustCrypto `scrypt`,它对 `r` 1 配 `n` 2^18 答参数不合(`log2 n` 须小于 `16 r`),Web3 Secret Storage 旧维基那份 `r` 1、`p` 8 的向量因此读不动;ethereum.org 现页那份 scrypt 向量在本实现与 `cast` 两处都答 MAC 不对,记为发现。口令与助记词从档读时去掉末尾至多一个换行(`file_secret`)。
68. **基准两成员名导出**(r5 批二,条 3 纯只增):`audit::BASIS_CHAINS`、`audit::BASIS_BARE_TX`;K3 挑选造空基准与 V 组 `names::I_CHAINS`、`I_BARE_TX` 改引;A3 `basis.rs` 与 K2 `view.rs` 里还是裸字面(不在 r5 批二墙内)。r6 批一:两枚入字面簿(角色 `basis-member`),检查器第⑪读现比登记簿与这两枚常量;A3 与 K2 的裸字面改引归批二(同类申报)。
69. **字面簿检查器八读**(r5 批二):⑦`entries` 的角色属 `roles` 闭集;⑧派生路径表非空、形合、项目号等于 `project`、路径两两不同、`(family, role)` 两两不同;产品按角色取派生路径只经 `literals::derivation_path`(嵌入登记簿)。
70. **按类型取体的公有口**(r5 批三,条 3 纯只增):`envelope::Entry::body_of_type(entry_type)`,`entryType` 逐字节相等才答 `body`;既有两成员的读法,零新判定。消费者:V1 `audit --amanah` 的非 amanah 拒、V3 `amanah-resolve` 与 `guild-join` 读本席账本里的 amanah 条目、V4 `panel engage` 与 `bench-report` 读成员账本里的行会轨条目。
71. **keystore 读档的 KDF 代价上界**(r5 批四 code-review):代价参数出自档,先判界再做功:scrypt 内存 `128·r·n` 至多 1 GiB、`p` 至多 16、总功至多 4 GiB 字节,pbkdf2 至多一千万轮,越界即 `keystore-params`;常见钱包写的档都在界内。
72. **秘密档的行尾**(r5 批四 code-review):`file_secret` 去掉末尾至多一个 `\r\n` 或 `\n`。
73. **字面簿规则⑧的段读法**(r5 批四 code-review):派生路径的段按产品派生那一份 `cryptox::parse_path` 读(`{i}` 只许是整个序段,以 0 代入),检查器放过的每一行产品都派生得出;路径两两不同按派生出的路径集判,以 `{i}` 收尾的一行与同前缀的任何一行相交即重复。
74. **锚的三形与记录第八员**(r6 批一,修订后 §9.1、§9.2):形名三枚与签名两枚同住 `kit.rs`(`FORM_REGISTRY`、`FORM_CARRIED`、`FORM_BARE`、`ANCHORED_SIGNATURE`、`ANCHORED_CARRIED_SIGNATURE`);登记表形带任何 data 字节即不是锚,携带形的 data 在「是锚」定下之后才读,读法住 `kit::carried_anchor`(一个 `bytes` 值的 ABI 编码加 `sha256` 等于 topic 2,两样缺一即「立锚而不携带」)。`Record` 第八员 `entry` 只现于携带形且确实携带了字节的记录,比的是字节不是拼法;`sha256(entry)` 不等于 `hash` 即不是记录。
75. **pile 双源与 CARRIED_MALFORMED**(r6 批一,修订后 §8.1、§8.4):带 `entry` 的记录的字节一律入 pile(不问 sender、verdict、是否被世系修剪),世系与 §7.4 自此从并后的多重集算;两支判定住 `audit::carried_malformed`,只对「counted 且未被世系修剪」的记录所携带的字节串判,一字节串一条,两支互斥,是账本条目的什么也不记。标签第 2 条因此对「自己携带了字节的 UNPROVEN 记录」不举 `UNAVAILABLE`。
76. **基准 `registries` 的类项**(r6 批一,修订后 §9.4):元素两式(hex20 串、恰一成员 `codeHash` 的类项),串在前字节序升、类项在后按 `codeHash` 字节序升、两半各无重复,住 `audit::registries` 一扇;同链相邻两窗的「`registries` 相等」按元素逐个比,类项也算。K2 的覆盖谓词只数元素个数,两式都算(`view.rs`)。
77. **字面簿检查器十一读**(r6 批一):⑩两枚 `Anchored` 签名,法 §9.1 按 topic 0 的原像那一形 `keccak256("…")` 现取的集、产品两枚常量、登记簿登记的签名集三者恰等(括号里只许字母数字逗号方括号,于是同一节里两 topic 的散文例句取不进来);⑪`basis-member` 角色恰等于 `audit::BASIS_CHAINS` 与 `audit::BASIS_BARE_TX`,且各在 §9.4 段里带反引号逐字出现。事件名不另写字面,从 `ANCHORED_SIGNATURE` 的头上取。
78. **K6 金库密码学落在 `cryptox` 的哪几处**(r7 批一):新出生籍功能 K6 的代码全住 `cryptox::vault` 一个子模块(第三方密码学仍只此一处)。`r` 为 8、`p` 为 1 钉死在 `KDF_R`、`KDF_P`;**上界不另写一个数** —— `max_log_n()` 现问 K1 那一扇 `keystore::scrypt_cost_bounded`(`r`、`p` 钉死之后它自己答 20),下界取 `keystore::WRITE_LOG_N`。标定值与档里读到的值都过同一扇 `admits`,撞上界即拒。
79. **两枚用途串不住码里**(r7 批一):`recovery` 与 `galeed-age/1` 的字面只住 K5 的 `FAMILY-LITERALS.md`,`cryptox::vault` 按 Desk 侧的用途名(`USE_RECOVERY` = `master-recovery`、`USE_AGE` = `age-identity`)经 `literals::derivation_purpose` 现取;读不出即答 `None`,调用者据此拒,永不以空用途串派生。
80. **age 身份串的 bech32 自己写**(r7 批一):`age` 的公有面只给「现取一枚随机身份」与「读一个身份串」,没有「由这三十二字节做一枚身份」;锁文件本轮不动版本、直接依赖只许升三件,故 bech32 编码写在 `cryptox` 里(与 `base58` 同类,是带校验的编码不是密码学)。正确性以 `age` 自己的序列化器作参照:编出来的串交给 `age` 解,它再写一遍,与我编的逐字节相同(`vault_tests::bech32_matches_age_own_spelling`)。
81. **两扇产品签名门**(r7 批一):`sign_as(key, b, seat, domain)` 按 (席位, 域) 两项放行(契约 2、铁律 8 的机器腿),`sign_registered(key, b, domain)` 只问域登记过没有(**某一席**可签它即放行)。两扇同一张表(`literals::embedded_domains_by_seat`)、同一具签名本体;域那一半的答案逐字相同,故既有调用方的字节一个也没动。V1、V2 与 `galeed-papers` 换到带席位那一扇归批二(钥源两路一并给出席位)。
82. **`sign_registered` 退役**(r7 批二):产品面只剩按 (席位, 域) 两项的 `sign_as` 一扇。V1 的 `mint`、V2 的四枚签发动词与 `galeed-papers` 的 `doc` 与 `gpp` 四处换门,`galeed-anchor` 的交易签名本就走另一路。`literals` 夹具那一支 `sign-registered` 留着只为把「域未登记」那一色答给 K5 行程,席位由 `seat_of_domain` 按域现答(登记簿把域按席位分列,故一域恰一席)。

## r11 批三 · 纯只增一口

- **`audit::broken_chain(report)`**:报告的标签是 `BROKEN_CHAIN` 即真。只导出既有常量上的一句判,不改任何读法。消费者:壳的锁写 `app::audit::writes_locked` 与命令行出口闸 `galeed_cli::exit::judge`(裁定 4.5 第 3 项:「断」一处判,V 组不拿标签作比)。
- **`literals::datum_text(name)` 与 `DATA_REGISTRY_CODE_HASH`**:登记簿 `data` 一节里串形常数的读口(与 `datum` 同读一份)。只导出既有嵌入档里的值,不改任何读法。消费者:app 身份设置里登记簿地址那一格的「保存即比对 codeHash」(`app::endpoints::save_registry`,设计版本监督第 6 条;裁定 4.5 第 1 项)。

## r16 重做批一 · 链上即账本(Court `6497f098`;本节所写取代上文各条里与之相左的读法:34、35、37、43、44、45、52、68、74、75、77)

- **审计两入**(§8):`audit::Input` 只 `{root, anchors, basis}`;`read_input` 读这三员,对象开放,其余成员不读。条目只来自记录的 `entry`(§8.1「携带的字节串」,`audit::carried` 一处);机器旁边持有的字节一律不入审计。
- **§8.1 新序**:收携带字节 → §4.3 → 世系(过 §4.3 的携带串上的不动点)→ 发出者不在世系的记录入 `DISCARDED` → 账本 = 世系内、且被某条未丢弃 `counted` 记录携带的条目。只被非计入记录携带的继任照样延世系,不移权(§8.6),其后条目照 §8.2 记 `AUTHORITY_MISMATCH`。
- **§8.5**:`MISSING` 是未丢弃 counted 记录的 hash 不在 `have` 者,不另扣任何集合;四种来路(无字节、字节非条目、条目世系外、字节只在非计入或被丢弃的记录上)都落在这一张表里。`ANCHORED` 每条账本条目一行。
- **报告十三员**,删 `unanchored`、`unavailable`;`unproven`、`void` 行在记录带字节处并 `entry`。标签 `BROKEN_CHAIN > UNPROVEN > GAPS > COMPLETE`,`UNPROVEN` 一处拼写(`LABEL_UNPROVEN` 即 `VERDICT_UNPROVEN`)。
- **基准只 `chains`**:`BASIS_BARE_TX` 撤,带 `bareTx` 的基准不是基准(NO_LABEL)。
- **§12.3 迁移**:新 spec 条目由世系钥携带上链即过不了 §4.3,入 `MALFORMED`、`MISSING`,并记 `CARRIED_MALFORMED`(硬,`BROKEN_CHAIN`);无专门分支,`carried_malformed` 一扇即答。
- **取证包一形**(§9.1、§9.6):`kit::judge` 只认 topic 0 为 `ANCHORED_CARRIED_SIGNATURE` 的日志,不读包的 `form` 成员;两 topic 的 `ANCHORED_SIGNATURE` 撤。`FORM_*` 三枚词留在 `kit.rs`,核内无读者,读者是捕获一侧(`galeed-anchor`)与命令行。
- **草稿走查**(`draft::walk`):本机条目集上跑 §8.2 与 §8.4 的成对一判,与审计同一具 `walk`、`equivocations`、`lineage`;答条目数、发现、是否有硬、头(最高 seq 与其各 id),不出标签。
- **条目上限**(§11 Size and transport):`entry::CARRIED_MAX` = 130,752 字节,由一笔 `anchorCarried` 的交易大小界推得(见常量注);读者是存储层 `galeed_store::ledger::ENTRY_MAX` 与发送层 `galeed_chain::send::calldata_max`,各引它;r16 17b 起不另立判函数与口(B102:`carriable` 无读者、口是空腿,已删)。
- **`amanah.quotes`**(§6.2):可选,须是对象,成员一概不判。

## r16 重做批三 · 链面(本节所写取代上文「取证包一形」一条里 `FORM_*` 三枚词那半句)

- **`FORM_*` 三枚词撤**:核内本无读者,捕获一侧(`galeed-anchor`)只剩一形,不再写包的 `form` 成员;命令行 `codes::Word` 照来家字面拼它的两枚词。
- **一具包含检验**(§9.1):`kit::contained` 公开,扫描器(`galeed_scan::scan::contained`)与取证包判定同读这一具;口 `ablate-k1-contained`(`galeed-scan` 的 `ablate-a3-scan` 转到它)。
- **发出者账户证明**(§9.6,B032):包可带 `emitterProof`(发出者账户对锚块 stateRoot 的证明),判出即答 `emitterCodeHash`,供按类项准入的收件人在自己的基准下判 §9.1 的准入;证明在而不立即什么也不立(`emitter-proof`),不当作没问。

- r16 重做批四:字面登记簿检查器对新法:⑨ 文书 kind 与保留前缀分两列核,⑩ `Anchored` 恰一签名,⑪ 基准恰一员 `chains`,⑬ 保留前缀是 galeed.roster/1 §8 与文书法两法前缀之并且无占名;法文锚点 `9.1 The form.`。条目字节拼装一具 `envelope::assemble`(B029,命令行 mint 与桌面 `entryx::seal` 同调)。
- r18 批一:`json::parse_tests_1_3`(§3.5 一至三查,复用 `parse` 与 `has_dup_key`,纯只增)开给命令行 `mcp` 读来自法外的消息行;消费者 `galeed-cli::mcp::take`。

## r18 批二 · 退任与迁法(Court `649120da`;本节取代上文 38、41 与各节里与之相左的读法)

65. **第八种条目 `migration`**:体表 `spec`(token)与 `statement_md` 两员;`KNOWN_TYPES` 八枚。
66. **退任(§7.4)**:每把钥的退任时刻是锚集里承载其所签 succession 或 migration 的记录中最小的 `blockTimestamp`(任何链、任何 sender 与 verdict)。世系只并入「至少一条承载记录在其签者未退任之时」的继任。`lineage_of` 吃整份审计输入;不看时刻的本地超集另名 `lineage_local`,手里没有记录时用,差在退任后签的继任(§7.4 不并入,它并入)。多数调用只取键集(取扫描钥一类);`standing_gate`、`of_member`、`admit` 三处拿它作判,留:手里没有记录判不了退任,差的那一形产品拒铸,只能从别机合入。
67. **trim(§8.1)**:sender 不在世系、或该记录时刻其 sender 已退任,即入 `DISCARDED`;`CARRIED_MALFORMED` 只认计入且未丢弃的记录。
68. **走查闭账(§8.2)**:某位置上有在任钥签的 migration,`auth` 置空,其后访到的条目皆 `AUTHORITY_MISMATCH`。草稿走查同一具。审计答形无顶层 `ok`,退任与闭账的翻转跑器看不见,不设消融口,由收敛闸(核语料)与 crate 单测量。
69. **退任同秒(§7.4,Court `24353685`,命名 2026-10-30)**:每把钥记退任时刻与**退任位**(`Retirement {at, seq}`:退任那一秒所载、过 §4.3、为继任或迁移、`judge` 为该钥的字节串里最小的 `seq`,任何链、任何 sender 与 verdict,`retirements_of` 一处建)。**已退任**两式(`has_retired` 一处判):块时晚于退任;或等于退任且该记录所载字节过 §4.3、`judge` 为该钥、`seq` 大于退任位。同秒里该钥退任位及以前所签、他钥所签、不过 §4.3 的字节、无字节的记录照早先记录计。trim 以记录 `sender` 问,世系接纳以继任的 `judge` 问,桌面三读者(`app::retire`)喂所见记录与所载字节问同一谓词。判据变化:前钥在交接那一秒所发、载其交接之后位置所签条目的记录,旧判保留(多半 `BROKEN_CHAIN`),新判丢弃(`DISCARDED`)。消融两口:`ablate-k1-retire`(两式皆灭)、`ablate-k1-retire-seq`(只灭同秒那一式)。
