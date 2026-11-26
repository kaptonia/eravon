# K3 · NOTES(courtevidence/1 留给实现的选择,逐条)

法文本 `base/docs/courtevidence-v1.md` 是判据的散文,冻结核(§8.5)是判据;本档只记法与 HARNESS-EVIDENCE.md 没有定、由本实现定的读法。验收形:`compare-evidence.py` seed 1 至 4 零分歧零预测偏差。

1. **`canonical(b)` 与 `subjectRef` 取 K1**。§2.3 第一步是 `galeed::json::accept`(§3.5 六测,八枚 token),`subjectRef` 是 `galeed::body::subject_ref`(galeed/1 §6.3 的表);本 crate 不另写正典器与主题表。
2. **§4.1 的成员判定用报告的两列**。pile 去重后,`entry_id`(sha256)不在报告 `excluded` 与 `malformed` 里的字节串即账本条目;条目的 `seq`、`body.recipient`、`body.effective` 从它自己的字节经 K1 `envelope::accept` 读回。
3. **§3.2 的 Bech32 自写在本 crate**。BIP-173 校验和与五位组换字节属编码,不调任何密码学 crate;`recipient_key` 答那三十二个字节,§3.2 的谓词与加密前的小阶点闸共用它。
4. **加密前两道闸**。`encrypt` 先过 §3.2 形,再拒小阶点:X25519 解码钥时最高位不读、再模 p,共享秘密全零恰是这七个拼法(0、1、两个八阶点、p − 1、p、p + 1,最高位去掉后比),一个不多一个不少。age crate 碰到全零共享秘密会 panic,§3.3 要响亮地拒而不中止,所以闸在交 `cryptox` 之前。拒答出缘由(`recipient-form` / `small-order`),不写任何密文。
5. **`gev` 在契约四命令外另有三条产品自证命令**:`age-roundtrip`、`age-decrypt`、`age-encrypt`(先例是 `gl1 kit`)。§8.1 说 §3 除收件者形外不入语料,往返与互通是生产者自己的测;契约外的其余命令名仍是误用。
6. **HARNESS 面**:`recipient-check` 的参数装不成 UTF-8 时必含 0x7E 以上的字节,答 `E_RECIPIENT`,不算误用;文件过十六 MiB 即误用;stdout 写不出退 2。
7. **旧密文永不重新加密**:本 crate 没有换收件者重加密的 API。
8. **`build` 只写字节**:清单的正典字节由 K1 写法产出;写出的字节成不成清单由 `check` 判,`build` 不替调用者判。

## r16 重做批二 · 链上即账本(Court `6497f098`;本节取代上文 2 与 4、8 里与之相左的读法)

9. **挑收件人读审计输入**:`pick(&Value)` 读不成即 `unread`;`pick_of(&Input)` 的绑定取报告 `anchored` 行所指、`entryType` 为 `keybinding` 的携带条目。§4.2 洞:`SEQ_GAP`,或 `missing`/`unproven` 行的 hash 不是任何账本条目、`MALFORMED`、`EXCLUDED` 行的 `entry_id`;只由 VOID 携带的后续绑定不造洞。`gap` 的答面带 `holes`(补扫什么),契约答形不带。
10. **文件项第五员 `zikaron`**:恰四员,判序在 `mediaType` 之后;`_md` 键在封闭表里照该表的拒名拒,字符集从不出场(由 `canonical(b)` 的散文子树规则自然得出)。
11. **生产者义务各一处**:`FileItem` 的摘要与长度只由 `FileItem::listed` 从所列字节算(成员不可写);文件名一判 `file_name`(skeleton、包内相对路、无 `\`、`:`、`.`/`..` 段、不收 `.DS_Store`),加文件那一刻判;`usable` 判收件人可用(形与小阶点),发绑定前与加密时同调;`seal` 只加密给挑出的 `Binding`(只由挑选造出),清单项列明文,拒即什么都不加密、不回落明文。
12. **`gev pick-recipient <audit.json>` 一参**:读得到的档一律有答;读不到的路径是误用。
