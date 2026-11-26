use eravon_galeed::Value;

use crate::{Why, why};

pub const FORMAT: &str = eravon_literals::literals::DOC_COMPLAINT;
pub const E_TEXT: &str = "E_COMPLAINT_TEXT";
pub const E_NO_GUILD: &str = "E_COMPLAINT_NO_GUILD";
pub const E_SIGNER: &str = "E_COMPLAINT_SIGNER";
pub const E_FILE: &str = "E_COMPLAINT_FILE";
pub const E_WRITE: &str = "E_COMPLAINT_WRITE";
pub const E_EXISTS: &str = "E_COMPLAINT_EXISTS";
pub const E_FORM: &str = "E_COMPLAINT_FORM";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Dispatch,
    Pass,
    AuditInput,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Dispatch, Kind::Pass, Kind::AuditInput, Kind::Other];

    pub const fn word(self) -> &'static str {
        match self {
            Kind::Dispatch => "dispatch",
            Kind::Pass => "pass",
            Kind::AuditInput => "audit-input",
            Kind::Other => "other",
        }
    }

    pub fn of(w: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.word() == w)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Buyer,
    Seller,
}

impl Role {
    pub const fn word(self) -> &'static str {
        match self {
            Role::Buyer => "buyer",
            Role::Seller => "seller",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub kind: Kind,
    pub name: String,
    pub bytes: Vec<u8>,
}

pub fn attachment(kind: Kind, path: &str) -> Result<Attachment, Why> {
    let p = std::path::Path::new(path);
    let name = p.file_name().and_then(|n| n.to_str()).filter(|n| !n.is_empty()).ok_or_else(|| why(E_FILE, format!("{path} 不是一个文件")))?.to_string();
    let bytes = std::fs::read(p).map_err(|e| why(E_FILE, format!("读不到 {path}:{e}")))?;
    Ok(Attachment { kind, name, bytes })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    pub chain_id: u64,
    pub instance: String,
    pub case_id: String,
    pub logic: String,
    pub judge: String,
    pub guild: Option<String>,
    pub complainant: String,
    pub role: Role,
    pub text: String,
    pub created_at: u64,
    pub attachments: Vec<Attachment>,
}

fn lower(s: &str) -> Value {
    Value::Str(s.trim().to_ascii_lowercase())
}

fn body(d: &Draft) -> Result<Vec<(String, Value)>, Why> {
    if d.text.trim().is_empty() {
        return Err(why(E_TEXT, "投诉内容是空的"));
    }
    let guild = destination(d)?;
    let atts = d
        .attachments
        .iter()
        .map(|a| {
            Value::Obj(vec![
                ("kind".into(), Value::Str(a.kind.word().into())),
                ("name".into(), Value::Str(a.name.clone())),
                ("sha256".into(), Value::Str(eravon_galeed::hash::hex_encode(&eravon_galeed::hash::sha256(&a.bytes)))),
                ("base64".into(), Value::Str(base64(&a.bytes))),
            ])
        })
        .collect();
    Ok(vec![
        ("v".into(), Value::Str(FORMAT.into())),
        ("chainId".into(), Value::Int(d.chain_id)),
        ("instance".into(), lower(&d.instance)),
        ("caseId".into(), lower(&d.case_id)),
        ("logic".into(), lower(&d.logic)),
        ("judge".into(), lower(&d.judge)),
        ("guild".into(), Value::Str(guild)),
        ("complainant".into(), Value::Obj(vec![("address".into(), lower(&d.complainant)), ("role".into(), Value::Str(d.role.word().into()))])),
        ("text".into(), Value::Str(d.text.clone())),
        ("createdAt".into(), Value::Int(d.created_at)),
        ("attachments".into(), Value::Arr(atts)),
    ])
}

fn destination(d: &Draft) -> Result<String, Why> {
    d.guild.as_deref().map(str::trim).filter(|g| !g.is_empty()).map(str::to_string).ok_or_else(|| why(E_NO_GUILD, format!("判官 {} 不属任何行会,无处投诉", d.judge)))
}


fn canonical(v: &Value) -> Result<Vec<u8>, Why> {
    eravon_galeed::canon::canonical(v).map_err(|t| why(E_FORM, t.name()))
}

pub fn personal_digest(m: &[u8]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(32 + m.len());
    pre.extend_from_slice(b"\x19Ethereum Signed Message:\n");
    pre.extend_from_slice(m.len().to_string().as_bytes());
    pre.extend_from_slice(m);
    eravon_galeed::hash::keccak256(&pre)
}

pub fn sign(d: &Draft, key: &[u8; 32]) -> Result<Vec<u8>, Why> {
    let mut members = body(d)?;
    let covered = canonical(&Value::Obj(members.clone()))?;
    let (sig, by) = eravon_terms::sig::sign_secp256k1(key, &personal_digest(&covered)).ok_or_else(|| why(E_SIGNER, "这三十二字节不是一枚 secp256k1 私钥"))?;
    if !by.eq_ignore_ascii_case(d.complainant.trim()) {
        return Err(why(E_SIGNER, format!("签字的钥是 {by},投诉人是 {}", d.complainant)));
    }
    members.push(("sig".into(), Value::Str(sig)));
    canonical(&Value::Obj(members))
}

pub fn signer(file: &[u8]) -> Result<(String, String), Why> {
    let bad = |d: &str| why(E_FORM, d.to_string());
    let v = eravon_galeed::canon::read_open(file).ok_or_else(|| bad("读不成 JSON"))?;
    let m = v.as_obj().ok_or_else(|| bad("不是一个对象"))?;
    if v.get("v").and_then(Value::as_str) != Some(FORMAT) {
        return Err(bad("v 不是 eravon.complaint/1"));
    }
    let sig = v.get("sig").and_then(Value::as_str).ok_or_else(|| bad("没有 sig"))?;
    let rest: Vec<(String, Value)> = m.iter().filter(|(k, _)| k != "sig").cloned().collect();
    let covered = canonical(&Value::Obj(rest))?;
    let named = v.get("complainant").and_then(|c| c.get("address")).and_then(Value::as_str).ok_or_else(|| bad("没有 complainant.address"))?.to_string();
    let by = eravon_terms::sig::verify_secp256k1(&personal_digest(&covered), sig, &named.to_ascii_lowercase()).map_err(|f| bad(f.token.name()))?;
    Ok((by, named))
}

pub fn file_name(d: &Draft) -> String {
    let inst = d.instance.trim().trim_start_matches("0x").to_ascii_lowercase();
    format!("complaint-{}-{}.json", inst.get(..8).unwrap_or(&inst), d.created_at)
}

pub fn write(dir: &str, d: &Draft, key: &[u8; 32]) -> Result<String, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let bytes = sign(d, key)?;
    let at = std::path::Path::new(dir);
    if !at.is_dir() {
        return Err(why(E_WRITE, format!("{dir} 不是一个文件夹")));
    }
    let path = at.join(file_name(d)).to_string_lossy().to_string();
    match eravon_store::atomic::land_new_only(&path, &bytes) {
        Ok(true) => {}
        Ok(false) => return Err(why(E_EXISTS, format!("{path} 已经有了,没有写"))),
        Err(e) => return Err(why(E_WRITE, e.detail())),
    }
    let back = eravon_store::atomic::read_file(&path).map_err(|e| why(E_WRITE, e.detail()))?;
    let (by, named) = signer(&back)?;
    if back != bytes || !by.eq_ignore_ascii_case(&named) {
        return Err(why(E_FORM, format!("{path} 读回来与写的不符")));
    }
    Ok(path)
}

pub fn base64(b: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for chunk in b.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);
        let v = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | buf[2] as u32;
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(char::from(A[((v >> (18 - 6 * k)) & 0x3f) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7u8; 32];

    fn me() -> String {
        eravon_terms::sig::sign_secp256k1(&KEY, &[0u8; 32]).expect("钥").1
    }

    fn draft(guild: Option<&str>) -> Draft {
        Draft {
            chain_id: 11155111,
            instance: "0xAbCd00000000000000000000000000000000Ef01".into(),
            case_id: "0x00000000000000000000000000000000000000000000000000000000000000C5".into(),
            logic: "0x1111111111111111111111111111111111111111".into(),
            judge: "0x2222222222222222222222222222222222222222".into(),
            guild: guild.map(str::to_string),
            complainant: me(),
            role: Role::Buyer,
            text: "判官接单后\n过了裁决期限也不裁。".into(),
            created_at: 1794240000,
            attachments: vec![
                Attachment { kind: Kind::Dispatch, name: "dispatch.json".into(), bytes: b"{\"d\":1}".to_vec() },
                Attachment { kind: Kind::AuditInput, name: "audit.json".into(), bytes: vec![0, 1, 2, 250, 251] },
            ],
        }
    }

    const GUILD: &str = "galeed:guild:11155111:0x3333333333333333333333333333333333333333";

    #[test]
    fn a_complaint_carries_the_format_in_canonical_bytes_signed_by_the_complainant() {
        let d = draft(Some(GUILD));
        let b = sign(&d, &KEY).expect("签得出");
        let s = String::from_utf8(b.clone()).expect("UTF-8");
        let v = galeed::json::read_open(&b).expect("读得成");
        assert_eq!(galeed::json::canonical(&v).expect("规范"), b, "整档按规范序列化写出");
        assert!(s.starts_with("{\"attachments\":[{\"base64\":"), "键按字典序:{s}");
        assert!(!s.contains(": ") && !s.contains(", "), "无空白:{s}");
        assert_eq!(v.get("v").and_then(|x| x.as_str()), Some("eravon.complaint/1"));
        assert_eq!(v.get("chainId").and_then(|x| x.as_int()), Some(11155111));
        assert_eq!(v.get("instance").and_then(|x| x.as_str()), Some("0xabcd00000000000000000000000000000000ef01"));
        assert_eq!(v.get("judge").and_then(|x| x.as_str()), Some("0x2222222222222222222222222222222222222222"));
        assert_eq!(v.get("logic").and_then(|x| x.as_str()), Some("0x1111111111111111111111111111111111111111"));
        assert_eq!(v.get("guild").and_then(|x| x.as_str()), Some(GUILD));
        assert_eq!(v.get("createdAt").and_then(|x| x.as_int()), Some(1794240000));
        assert_eq!(v.get("text").and_then(|x| x.as_str()), Some("判官接单后\n过了裁决期限也不裁。"));
        let c = v.get("complainant").expect("complainant");
        assert_eq!(c.get("address").and_then(|x| x.as_str()), Some(me().as_str()));
        assert_eq!(c.get("role").and_then(|x| x.as_str()), Some("buyer"));
        let atts = v.get("attachments").and_then(|x| x.as_arr()).expect("attachments");
        assert_eq!(atts.len(), 2);
        assert_eq!(atts[0].get("kind").and_then(|x| x.as_str()), Some("dispatch"));
        assert_eq!(atts[0].get("name").and_then(|x| x.as_str()), Some("dispatch.json"));
        assert_eq!(atts[0].get("base64").and_then(|x| x.as_str()), Some("eyJkIjoxfQ=="));
        assert_eq!(atts[1].get("kind").and_then(|x| x.as_str()), Some("audit-input"));
        assert_eq!(atts[1].get("base64").and_then(|x| x.as_str()), Some("AAEC+vs="));
        let sha = galeed::hexform::encode(&galeed::cryptox::sha256(&[0, 1, 2, 250, 251]));
        assert_eq!(atts[1].get("sha256").and_then(|x| x.as_str()), Some(sha.as_str()));
        let rest: Vec<(String, galeed::json::Value)> = v.as_obj().unwrap().iter().filter(|(k, _)| k != "sig").cloned().collect();
        let covered = galeed::json::canonical(&galeed::json::Value::Obj(rest)).unwrap();
        let digest = galeed::sign::digest(&covered);
        let raw = galeed::hexform::decode_form(v.get("sig").and_then(|x| x.as_str()).unwrap());
        assert_eq!(raw.len(), 65);
        let (mut r, mut s2) = ([0u8; 32], [0u8; 32]);
        r.copy_from_slice(&raw[..32]);
        s2.copy_from_slice(&raw[32..64]);
        assert!(raw[64] == 27 || raw[64] == 28, "v 照 personal_sign 写 27 或 28");
        let by = galeed::cryptox::recover_address(&digest, &r, &s2, raw[64] - 27).expect("恢复得出");
        assert_eq!(galeed::hexform::encode(&by), me(), "sig 恢复出投诉人");
        assert_eq!(signer(&b).expect("读回"), (me(), me()));
    }

    #[test]
    fn no_guild_no_file_and_an_empty_text_or_another_key_signs_nothing() {
        let dir = std::env::temp_dir().join(format!("eravon-complaint-none-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_string_lossy().to_string();
        for g in [None, Some(" ")] {
            assert_eq!(write(&d, &draft(g), &KEY).map_err(|e| e.0), Err(E_NO_GUILD.to_string()));
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0, "无行会一个字节也不写");
        let mut empty = draft(Some(GUILD));
        empty.text = "  \n".into();
        assert_eq!(sign(&empty, &KEY).map_err(|e| e.0), Err(E_TEXT.to_string()));
        assert_eq!(sign(&draft(Some(GUILD)), &[9u8; 32]).map_err(|e| e.0), Err(E_SIGNER.to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_file_lands_once_and_an_edited_file_no_longer_holds_its_signature() {
        let dir = std::env::temp_dir().join(format!("eravon-complaint-land-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_string_lossy().to_string();
        let dr = draft(Some(GUILD));
        let path = write(&d, &dr, &KEY).expect("写得出");
        assert!(path.ends_with("complaint-abcd0000-1794240000.json"), "{path}");
        let first = std::fs::read(&path).unwrap();
        assert_eq!(write(&d, &dr, &KEY).map_err(|e| e.0), Err(E_EXISTS.to_string()));
        assert_eq!(std::fs::read(&path).unwrap(), first);
        let edited = String::from_utf8(first).unwrap().replace("也不裁", "也裁了");
        match signer(edited.as_bytes()) {
            Ok((by, named)) => assert!(!by.eq_ignore_ascii_case(&named)),
            Err(e) => assert_eq!(e.0, E_FORM),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    const SAMPLE_SIG: &str = "0xd97f1e7af5a84c11c26e352db438b1c1e64be8e5851a2cc06b81b0ca6e6399490db17279f2428a0414b678ffbcd04f5a831f22078ec9809c331f5f2d69106a3a1b";

    #[test]
    fn a_sample_complaint_has_these_bytes_and_this_signature() {
        let d = Draft {
            chain_id: 11155111,
            instance: "0x00000000000000000000000000000000000000a1".into(),
            case_id: "0x00000000000000000000000000000000000000000000000000000000000000c5".into(),
            logic: "0x00000000000000000000000000000000000000b2".into(),
            judge: "0x00000000000000000000000000000000000000d4".into(),
            guild: Some("galeed:guild:11155111:0x00000000000000000000000000000000000000e5".into()),
            complainant: me(),
            role: Role::Seller,
            text: "a\"b\\c\n\u{1}é中".into(),
            created_at: 1794240000,
            attachments: Vec::new(),
        };
        let body = format!(
            "{{\"attachments\":[],\"caseId\":\"0x00000000000000000000000000000000000000000000000000000000000000c5\",\"chainId\":11155111,\"complainant\":{{\"address\":\"{}\",\"role\":\"seller\"}},\"createdAt\":1794240000,\"guild\":\"galeed:guild:11155111:0x00000000000000000000000000000000000000e5\",\"instance\":\"0x00000000000000000000000000000000000000a1\",\"judge\":\"0x00000000000000000000000000000000000000d4\",\"logic\":\"0x00000000000000000000000000000000000000b2\"",
            me()
        );
        let tail = "\"text\":\"a\\\"b\\\\c\\n\\u0001é中\",\"v\":\"eravon.complaint/1\"}";
        let b = sign(&d, &KEY).expect("签得出");
        let s = String::from_utf8(b.clone()).unwrap();
        let sig = galeed::json::read_open(&b).unwrap().get("sig").and_then(|x| x.as_str()).unwrap().to_string();
        assert_eq!(s, format!("{body},\"sig\":\"{sig}\",{tail}"), "整档字节");
        let covered = format!("{body},{tail}");
        let raw = galeed::hexform::decode_form(&sig);
        let (mut r, mut s2) = ([0u8; 32], [0u8; 32]);
        r.copy_from_slice(&raw[..32]);
        s2.copy_from_slice(&raw[32..64]);
        assert!(raw[64] == 27 || raw[64] == 28);
        let by = galeed::cryptox::recover_address(&galeed::sign::digest(covered.as_bytes()), &r, &s2, raw[64] - 27).expect("恢复");
        assert_eq!(galeed::hexform::encode(&by), me());
        assert_eq!(sig, SAMPLE_SIG, "样档签名");
    }

    #[test]
    fn base64_is_the_standard_padded_alphabet() {
        for (b, s) in [(&b""[..], ""), (b"f", "Zg=="), (b"fo", "Zm8="), (b"foo", "Zm9v"), (b"foob", "Zm9vYg=="), (b"fooba", "Zm9vYmE="), (b"foobar", "Zm9vYmFy"), (&[0xfb, 0xff][..], "+/8=")] {
            assert_eq!(base64(b), s);
        }
    }
}
