pub(crate) fn request_head(method: &str, target: &str, host: &str, lines: &str) -> String {
    format!("{method} {target} HTTP/1.1\r\nHost: {host}\r\n{lines}\r\n")
}
