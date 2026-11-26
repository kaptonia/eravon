#[test]
fn an_entropy_error_names_its_interface_and_source() {
    eravon_os::pretend_entropy_fails(true);
    let one = eravon_os::random(32);
    let mut buf = [0u8; 16];
    let two = eravon_os::fill_random(&mut buf);
    eravon_os::pretend_entropy_fails(false);
    for e in [one.err().expect("refused, never a weaker source"), two.err().expect("refused")] {
        let said = e.to_string();
        assert!(said.starts_with(&format!("entropy {}: ", eravon_os::ENTROPY_SOURCE)), "{said}");
        assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied, "the kind kept");
    }
    assert_eq!(buf, [0u8; 16], "nothing filled");
    assert_eq!(eravon_os::random(32).expect("read again once not pretended").len(), 32);
}
