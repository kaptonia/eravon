mod common;

use common::{Home, PIN};

const NOW: &str = "1758412900";
const NODES: [&str; 4] = ["--chain", "11155111", "--endpoint", "http://127.0.0.1:9"];
const SOMEONE: &str = "0x0000000000000000000000000000000000000001";
const INSTANCE: &str = "0x00000000000000000000000000000000000000aa";

fn given(verb: &str, file: &str) -> Vec<String> {
    let bare = verb.split_once('-').map(|(_, v)| v).unwrap_or(verb);
    let v: Vec<&str> = match bare {
        "terms-hash" | "terms-check" | "link-encode" => vec!["--file", file],
        "link-decode" => vec!["--link", "x"],
        "choose" => vec!["--link", "x", "--candidate", "0"],
        "vault-derive" => vec!["--role", "x"],
        "vault-rotate" => vec!["--continue", "true"],
        "history" => vec!["--who", SOMEONE],
        "product-show" | "product-live" => vec!["--id", "x"],
        "candidate-show" => vec!["--candidate", SOMEONE],
        "order-show" | "show" | "offers" => vec!["--instance", INSTANCE],
        "settle-sign" => vec!["--settle", "x"],
        "wallet-tx" => vec!["--tx", "0x00"],
        _ => vec![],
    };
    v.into_iter().map(str::to_string).collect()
}

fn both_seats(tag: &str) -> Home {
    let h = Home::new(tag);
    let seller = h.run(&["seller-seat-set", "--data", h.data_s(), "--writer", "t", "--now", NOW, "--passcode", PIN], |_| {});
    assert_eq!(seller.code, 0, "the seller seat is laid out: {} {}", seller.text, seller.err);
    h
}

fn run_locked(h: &Home, verb: &str, file: &str) -> (i32, String, String, String) {
    let mut args: Vec<String> = vec![verb.to_string(), "--data".into(), h.data_s().to_string(), "--now".into(), NOW.into()];
    args.extend(NODES.iter().map(|x| x.to_string()));
    args.extend(given(verb, file));
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let r = h.run(&refs, |_| {});
    (r.code, r.get("reason"), r.get("downToken"), format!("{} {}", r.text, r.err))
}

#[test]
fn a_verb_off_the_door_answers_locked_and_one_on_it_needs_the_vault() {
    let h = both_seats("door-rows");
    let file = h.root.join("terms.md");
    std::fs::write(&file, b"# terms\n").expect("a file");
    let file = file.to_str().expect("utf-8").to_string();
    let homed = |v: &str| eravon_cli::verbs::HOME.iter().any(|(r, _)| *r == v);
    let mut here = 0;
    let mut sealed = 0;
    for verb in eravon_cli::verbs::VERBS.iter().copied().filter(|v| eravon_cli::verbwire::split(v).is_some()) {
        let (seat, bare) = eravon_cli::verbwire::split(verb).expect("a seat's verb");
        assert_ne!(homed(verb), eravon_cli::schema::runs_here(verb), "{verb}: a row of the door, or run here, exactly one");
        if eravon_cli::schema::runs_here(verb) {
            let (code, reason, token, said) = run_locked(&h, verb, &file);
            assert!(code != 2 && token != "E_VAULT_LOCKED" && reason != "E_VAULT_LOCKED", "{verb} answers on a locked data root: {said}");
            here += 1;
        } else if !eravon_cli::schema::writes(seat, bare) && !eravon_cli::schema::sends(seat, bare) && bare != "seal" || bare == "vault-rotate" {
            let (_, _, token, said) = run_locked(&h, verb, &file);
            assert_eq!(token, "E_VAULT_LOCKED", "{verb} opens the vault, so it is the door's on that line: {said}");
            sealed += 1;
        }
    }
    assert_eq!((here, sealed), (11, 30), "eleven run here; the reads of sealed data, the countersignatures, the complaints and the rotations need the vault");
}
