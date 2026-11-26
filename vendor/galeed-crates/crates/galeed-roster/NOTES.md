# K4 · NOTES(galeed.roster/1 留给实现的选择,逐条)

法文本 `base/docs/galeed-roster-v1.md` 是判据的散文,冻结核(§10.5)是判据;本档只记法与 HARNESS-ROSTER.md 没有定、由本实现定的读法。验收形:`compare-roster.py` seed 1 至 4 零分歧零预测偏差。

1. **`uri` 以串承载**。`Publication.metadata_uri` 是 `String`;传输 hex 解出的八位组若非 UTF-8,以替换字符入串。§2.1 第 1 步只认二十二个 ASCII 前缀字节,第 2 步只放行 0x21 到 0x7E 与 `%` 转义,原字节里任何非 UTF-8 片段与换进来的替换字符(0xEF 0xBF 0xBD)在这两步答同一个 `E_URI`,前缀是否成立也不因替换改变,所以答案不变。
2. **§3.2 各行内部不排序**。`E_ROSTER_JUDGES`、`E_ROSTER_BENCH`、`E_ROSTER_GUILDTERMS` 各覆盖几项测试,法说行内无序;本实现照表列顺序测,token 同一枚。
3. **`bench` 与 `guildTerms` 以读者读出的原值承载**。解码时判完形;答案与解析要读时经 `bench_form` 现取读法,`guildTerms` 按 §3.3 只带 `sha256`、`version` 与在场的 `uri`、`quotes`。
4. **§3.2 的整数拼法与 §6.1、场所键的首段、HARNESS 的 `<chainId>` 共用一处**:`int_spelling` 让 K1 的读者判数形,再加「整串全是数字」一条,读者允许的首尾空白由这一条排除。
5. **一次解析之内的备忘**。`guild_of` 里同一地址(名录重复列名、验证人与判官同址)只走一趟 `resolveIdentity`;备忘只活在那一次调用里,跨调用零缓存,调用者在行动前重解析(§7)。
6. **HARNESS 面**:文件过十六 MiB 即误用;参数不是 UTF-8 即误用;stdout 写不出退 2。命令只有契约六条,别的一律误用。
7. **V 组读面的投影**。只剩一个:V2 `resolve-bench` 读的 `bench_three`,把约法 bench 指针 `(chainId, 会地址)` 拼成指针串交 §5.5 `bench_value`,判定不在投影里。V4 名录各动词直接读 `guild_of`、`decode`、`Refusal::name` 与 `Row`。
8. **§7 的义务不在本 crate**。名录字节只由 K1 写法产出、行会工具的拒发清单、池形签字门,都是 `[O]`,由产出与消费的动词守;本 crate 只提供它们要调的谓词。

9. **§7 报价键的文法住本 crate**（r6 批四加，纯只增）：`quote_key` 读四式（二段或三段，中段 hex20 或 hex32，末段在即 hex20），拒各有名（`E_QUOTE_KEY_PARTS|CHAIN|ITEM|TOKEN`）；`quote_key_string` 是它的反向，`quote_keys_address` 与 `quote_keys_standard` 给出 §7 那一句的先后（带币先于不带币，地址先于码）。本 crate 的行为零改：名录解码不调它们，它们只给读键的动词调。消费者：V3 `quote_of` 与 `quote_declarations` 两处。

## r16 重做批二 · 登记簿法(Court `6497f098`;本节取代上文 1、2、3 与 6 里与之相左的读法)

10. **发布即 `(emitter, blockNumber, logIndex, document, word)`**:`document` 是八位组(`Vec<u8>`),不是文本;§2.1 四步 `E_HASH`、`E_JSON`、`E_KIND`、按 kind 分派。数据 URI 与百分号解码一路删(`galeed::datauri` 随删),`E_URI` 不再有。
11. **`candidateOf` 一处**:`Logs::candidate_of(a, Kind)` 是唯一的「取一类最新一份」;§2.1 前三步读不出的发布是每一类的候选(`kind_read` 一处读三步,`of_kind` 一处判前缀)。名册、判官文档、行会判法、服务记录(papers §7a.3 经它)各读各的。
12. **判官文档 basis 由 `galeed/1` 核判**:`basis_form` 把它装进根为任一 hex20、锚集为空的审计输入交 `audit::read_input`,不收即 `E_JUDGE_LEDGER`;本 crate 不重写一条 §9.4 规则。
13. **行会判法的词由读者现算**:`text_word(text_md)` = `sha256`(UTF-8);写者存在旁边的什么值都不读。
14. **`quoteOf` 五键**:`quote_order` 定先后(地址先于码、带币先于不带、裸链键末),`quote_of` 只认第一把在场的键。`quote_key`(四式、中段必在)仍给读声明表的动词用,裸链键一式与收窄的合读归批四 `quote_read`(名7)。
15. **`Followed`**:继任解析答 `{listed, final, trail, broken(token, at)}` 一形(`follow`、`Row::followed`),各面显示 trail 与走断原因都取它(批五 `rail`)。
16. **`guildTerms` 不再是名册表的一员**:读者把它当表外成员(§2.2);命令行与桌面写它的旧路归批四、批五(`galeed_cli::names::R_GUILD_TERMS`)。
17. **HARNESS 面十条命令**:`publication` 第三参 `<kind>`;新 `judge`、`resolve-judge`(三参,不读 now)、`covenant`、`quote`;日志档的读者 `harness::logs_of` 一处,`gpp` 的 service 两命令也经它读。
18. **§5.10 五式的场所键读法一处**(r16 17b,纯只增):`venue_key` 读五式(一段即裸链键 `VenueKey::Chain`,二、三段经 `quote_key`),`quotes_venue(q, chain, contract, codeHash?)` 判「这一场所报没报价」(裸链键报其链上每一场所)。消费者:命令行承接闸 `v4::panel::quoted_target`、行会轨声明 `v3::quote_declarations`(覆盖经它);订阅(`app::watch::from_quote_keys`)取地址、码两式与判法 `venues`,裸键不列。

## r18 批二 · 换钥簿与行会名片(Court `649120da`;本节取代上文 3、15、17 与各节里与之相左的读法)

19. **日志集四表**:`Logs` 加 `declarations`(`Declared`)与 `accessions`(`Acceded`);`harness::logs_of` 一处读四表,四表各在、各元素照形、四表合起来 `(blockNumber, logIndex)` 不重,否则误用。
20. **§4.2 重放一具 `Book`**:root/head/pending/entered 照法逐条走;`current`、`current_at`(严格在位置之前)、`record_keys`(根在前、依承接序)、`pending`、`entered`、`idle`(不绑不计的记录)都问它。`resolve_identity` 是全函数:final 即 current,trail 是 keys 里从所问之址到 final 之前,attempts 是 trail 与 final 各钥的声明里未被计数承接取走的,按重放序。旧的六十四跳、`E_CYCLE`、`E_TOO_DEEP` 与 `galeed.succession/1` 种类都不在本法里。
21. **§3c 行会名片与 §5.11 `guild_card_of`**:四答 `CARD`/`CARD_NONE`/`CARD_UNREAD`/`CARD_MISMATCH`,不跟簿;`resolve-guild` 每答都带 `card` 与 `issued`,不读时钟,无到期步。
22. **名录表六键**:`expires`、`name`、`terms` 出表,写了的作表外成员按键列进 `unknownFields`。`ROSTER_TABLE` 公开,命令行的构建与驱动门引它,不另抄一份。
23. **bench 与 deny 按整份记录**:§5.4 第 7 步与 §5.5 数组行取所列之址记录的全部钥(所列之址之前的钥在内);§5.6 的 `deny` 比整份记录。
24. **HARNESS 十二命令**:`current-at`、`guild-card` 新立;`resolve-guild`、`bench`、`candidates` 不带时刻;`resolve-identity`、`resolve-judge` 的 `<chainId>` 只验拼法;kind 五词为 roster、guildcard、judge、guildamanah、verifier。
25. **消融口**:`ablate-k4-identity`(承接不验是否为 holder 的待认领钥)、`ablate-k4-guild`(名录不比 chainId)、`ablate-k4-bench`(只取所列之址往后)、`ablate-k4-candidates`(deny 只比所列之址往后)。`ablate-k4-current-at`(`current_at`)与 `ablate-k4-guild-card`(`guild_card`)两口,K4 两道 `k4-current-at`、`k4-guild-card` 经答员(`members`)读其答形。
