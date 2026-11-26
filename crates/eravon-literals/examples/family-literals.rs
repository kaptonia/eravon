fn main() {
    use std::io::Write;
    let _ = std::io::stdout().write_all(&eravon_literals::registry::family_json());
}
