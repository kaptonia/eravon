#[path = "solsrc.rs"]
mod solsrc;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let here = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest dir"));
    let base = here.join("../../base");
    let mut files = sol_files(&base.join("eravon-standard/src"));
    files.push(base.join("galeed-core/contracts/src/GaleedRegistry.sol"));
    files.push(base.join("galeed-core/contracts/src/GaleedSuccessionBook.sol"));
    write(&files, "abi_gen.rs", true);
    println!("cargo:rerun-if-changed={}", here.join("past").display());
    let mut gens: Vec<PathBuf> = std::fs::read_dir(here.join("past")).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    gens.sort();
    for g in gens {
        let name = g.file_name().and_then(|n| n.to_str()).expect("a generation's directory is named in ASCII").to_string();
        write(&sol_files(&g.join("src")), &format!("abi_gen_past_{name}.rs"), false);
    }
}

fn sol_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{} reads: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sol"))
        .collect();
    files.sort();
    files
}

fn write(files: &[PathBuf], leaf: &str, with_variants: bool) {
    let sources: Vec<(String, String)> = files
        .iter()
        .map(|p| {
            println!("cargo:rerun-if-changed={}", p.display());
            (p.display().to_string(), std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{} reads: {e}", p.display())))
        })
        .collect();
    println!("cargo:rerun-if-changed=solsrc.rs");
    let items = solsrc::read(&sources);
    let mut out = String::from("// Written by build.rs from the Solidity source; not edited by hand.\n\n");
    let mut names: Vec<String> = Vec::new();
    for it in &items {
        let c = solsrc::upper_snake(&it.contract);
        let n = solsrc::upper_snake(&it.name);
        let name = match it.kind {
            "function" => format!("{c}_{n}"),
            "event" => format!("{c}_EV_{n}"),
            _ => format!("{c}_ERR_{n}"),
        };
        assert!(!names.contains(&name), "two items take the constant name {name} (an overload, or two contracts' names meeting)");
        names.push(name.clone());
        writeln!(out, "/// `{}` {} of `{}`.\npub const {name}: &str = {:?};", it.name, it.kind, it.contract, it.sig).expect("writes");
        if it.kind == "function" {
            writeln!(out, "/// What `{}` returns, as a tuple of canonical types.\npub const {name}_RETURNS: &str = {:?};", it.name, it.returns).expect("writes");
            for (i, rn) in it.return_names.iter().enumerate() {
                if rn.is_empty() {
                    continue;
                }
                let col = solsrc::upper_snake(rn.trim_end_matches('_'));
                writeln!(out, "/// The place of `{rn}` among what `{}` returns.\npub const {name}_COL_{col}: usize = {i};", it.name).expect("writes");
            }
        }
        if it.kind == "event" {
            writeln!(out, "/// Which parameters of `{}` are indexed.\npub const {name}_INDEXED: &[bool] = &{:?};", it.name, it.indexed).expect("writes");
        }
    }
    for (contract, name, tuple, fields) in solsrc::structs(&sources) {
        let c = solsrc::upper_snake(&contract);
        let n = solsrc::upper_snake(&name);
        writeln!(out, "/// The struct `{contract}.{name}` as its ABI tuple.\npub const {c}_T_{n}: &str = {tuple:?};").expect("writes");
        for (i, f) in fields.iter().enumerate() {
            let col = solsrc::upper_snake(f.trim_end_matches('_'));
            writeln!(out, "/// The place of `{f}` among the members of `{contract}.{name}`.\npub const {c}_T_{n}_COL_{col}: usize = {i};").expect("writes");
        }
    }
    if with_variants {
        let init = items.iter().find(|i| i.contract == "Eravon" && i.name == "init").expect("Eravon declares init");
        writeln!(out, "/// `init` with the offer's last member dropped.\npub const ERAVON_INIT_OFFER_SHORT: &str = {:?};", drop_offer_last(&init.sig)).expect("writes");
    }
    out.push_str("\n/// Every item read: (contract, kind, signature, indexed).\npub const ITEMS: &[(&str, &str, &str, &[bool])] = &[\n");
    for it in &items {
        writeln!(out, "    ({:?}, {:?}, {:?}, &{:?}),", it.contract, it.kind, it.sig, it.indexed).expect("writes");
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join(leaf);
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("{} writes: {e}", dest.display()));
}

fn drop_offer_last(sig: &str) -> String {
    let b: Vec<char> = sig.chars().collect();
    let mut depth = 0;
    let mut start = None;
    for (i, c) in b.iter().enumerate() {
        match c {
            '(' => {
                depth += 1;
                if depth == 3 && start.is_none() {
                    start = Some(i);
                }
            }
            ')' => {
                if depth == 3 && start.is_some() {
                    let s = start.expect("opened");
                    let mut d = 0;
                    let mut last_comma = None;
                    for (j, cj) in b.iter().enumerate().take(i).skip(s + 1) {
                        match cj {
                            '(' => d += 1,
                            ')' => d -= 1,
                            ',' if d == 0 => last_comma = Some(j),
                            _ => {}
                        }
                    }
                    let k = last_comma.expect("the offer has two or more members");
                    let mut out: String = b[..k].iter().collect();
                    out.extend(b[i..].iter());
                    return out;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    panic!("init has no offer tuple");
}
