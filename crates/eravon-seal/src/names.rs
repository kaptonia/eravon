#[derive(Clone, Copy, Debug)]
pub enum Logical<'a> {
    Home(&'a str),
    Slot(&'a str),
    Products(&'a str),
    Orders(&'a str),
    Cases(&'a str),
    Packs(&'a str),
    Staging(&'a str),
}

impl Logical<'_> {
    fn tag(&self) -> &'static [u8] {
        use eravon_literals::literals::NAME_KINDS;
        match self {
            Logical::Home(_) => b"home",
            Logical::Slot(_) => b"slot",
            Logical::Products(_) => NAME_KINDS[1].as_bytes(),
            Logical::Orders(_) => NAME_KINDS[2].as_bytes(),
            Logical::Cases(_) => NAME_KINDS[3].as_bytes(),
            Logical::Packs(_) => NAME_KINDS[4].as_bytes(),
            Logical::Staging(_) => NAME_KINDS[5].as_bytes(),
        }
    }

    fn value(&self) -> String {
        let v = match self {
            Logical::Home(x) => x.trim().trim_start_matches("0x").to_ascii_lowercase(),
            Logical::Slot(a) | Logical::Products(a) | Logical::Orders(a) | Logical::Cases(a) | Logical::Packs(a) | Logical::Staging(a) => return a.to_string(),
        };
        v
    }

    fn width(&self) -> usize {
        match self {
            Logical::Home(_) => 40,
            _ => 64,
        }
    }
}

pub struct NameKey([u8; 32]);

impl NameKey {
    pub fn new(k: [u8; 32]) -> NameKey {
        NameKey(k)
    }

    pub fn name(&self, l: Logical) -> String {
        let mut msg = Vec::with_capacity(64);
        msg.extend_from_slice(l.tag());
        msg.push(0);
        msg.extend_from_slice(l.value().as_bytes());
        let mac = crate::crypto::hmac_sha256(&self.0, &msg);
        let hex = eravon::hexfmt::encode(&mac);
        hex.trim_start_matches("0x")[..l.width()].to_string()
    }

}

impl Drop for NameKey {
    fn drop(&mut self) {
        eravon_os::wipe(&mut self.0);
    }
}

pub fn is_home_name(n: &str) -> bool {
    n.len() == 40 && n.bytes().all(|b| b.is_ascii_hexdigit())
}
