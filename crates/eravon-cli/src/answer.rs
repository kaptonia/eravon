use eravon_literals::json::Out;

use crate::fault::Fault;

pub const EXIT_OK: u8 = 0;
pub const EXIT_MISUSE: u8 = 2;

pub fn ok(mut members: Vec<(&'static str, Out)>) -> Out {
    members.push(("ok", Out::B(true)));
    Out::O(members)
}

pub fn refuse(e: &Fault) -> Out {
    let mut m = vec![
        ("detail", Out::S(e.detail())),
        ("ok", Out::B(false)),
        ("token", Out::S(e.name().to_string())),
    ];
    m.extend(e.extra());
    Out::O(m)
}

pub fn answer<T>(r: Result<T, Fault>, f: impl FnOnce(T) -> Out) -> Out {
    match r {
        Ok(v) => f(v),
        Err(e) => refuse(&e),
    }
}

pub fn strs<S: AsRef<str>>(xs: impl IntoIterator<Item = S>) -> Out {
    Out::A(xs.into_iter().map(|x| Out::S(x.as_ref().to_string())).collect())
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(2 + b.len() * 2);
    s.push_str("0x");
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
