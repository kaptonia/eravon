pub type Why = (String, String);

pub struct Signer {
    key: Result<[u8; 32], Why>,
    live: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl Signer {
    pub fn of(session: Option<&eravon_vault::session::Session>, me: &str) -> Signer {
        let key = match session {
            None => Err(locked()),
            Some(s) => s.key_for(me).map_err(|e| (e.name().to_string(), e.detail())),
        };
        Signer { key, live: None }
    }

    pub fn locked() -> Signer {
        Signer { key: Err(locked()), live: None }
    }

    pub fn is_open(&self) -> bool {
        self.key.is_ok()
    }

    pub fn bound(mut self, live: std::sync::Arc<dyn Fn() -> bool + Send + Sync>) -> Signer {
        self.live = Some(live);
        self
    }

    pub fn live(&self) -> bool {
        self.live.as_ref().is_none_or(|f| f())
    }
}

pub fn stale() -> Why {
    ("E_GOODS_STALE".to_string(), "情况已变,这一手没有执行".to_string())
}

impl Drop for Signer {
    fn drop(&mut self) {
        if let Ok(k) = self.key.as_mut() {
            for b in k.iter_mut() {
                *b = 0;
            }
            let _ = std::hint::black_box(&k);
        }
    }
}

fn locked() -> Why {
    ("E_VAULT_LOCKED".to_string(), "已锁定".to_string())
}

pub fn sign_all(signer: &Signer, terms_bytes: &[u8]) -> Result<Vec<String>, Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let t = eravon_terms::doc::terms(terms_bytes).map_err(|f| (f.token.name().to_string(), "无法读取条款".to_string()))?;
    if !signer.live() {
        return Err(stale());
    }
    let key = signer.key.as_ref().map_err(Clone::clone)?;
    let total = (t.judges.len() + t.guilds.len()) as u64;
    let mut out = Vec::with_capacity(total as usize);
    for c in 0..total {
        let d = eravon_terms::derive::offer_digest_at(&t, c).map_err(|f| (f.token.name().to_string(), "无法生成订单要约".to_string()))?;
        let (sig, _addr) = eravon_terms::sig::sign_secp256k1(key, &d)
            .ok_or_else(|| ("E_GOODS_KEY".to_string(), "当前钥无效".to_string()))?;
        out.push(sig);
    }
    Ok(out)
}
