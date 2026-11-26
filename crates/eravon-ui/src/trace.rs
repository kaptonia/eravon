pub const FEATURE: &str = "H0";

static SINK: std::sync::OnceLock<fn(&str)> = std::sync::OnceLock::new();

pub fn install(sink: fn(&str)) {
    let _ = SINK.set(sink);
}

#[inline]
pub fn mark(feature: &str) {
    if let Some(f) = SINK.get() {
        f(feature);
    }
}
