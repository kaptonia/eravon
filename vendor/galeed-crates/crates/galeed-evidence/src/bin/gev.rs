//! `gev`:K3 的夹具二进制,四命令照 HARNESS-EVIDENCE.md,另加产品自证的三条 age 命令。stdout 恰一个正典
//! JSON 值、无尾换行;答出 0,误用 2。形闸与答案都在库里(`galeed_evidence::harness`),这里只起进程、写出、给退出码。
#![forbid(unsafe_code)]

use galeed::json;
use std::io::Write;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // 契约四命令的字形归 HARNESS-EVIDENCE.md;三条 age 命令是产品自有门,过门面那一道(每行 `ok`,拒行带 `refused`)。
    let contract = args.first().and_then(|a| a.to_str()).is_some_and(|c| galeed_evidence::harness::CONTRACT.contains(&c));
    let answered = galeed_evidence::harness::run(&args)
        .and_then(|v| if contract { Ok(v) } else { galeed::answer::stamp(v).map_err(|u| galeed_evidence::harness::Misuse(u.why().into())) });
    let code = match answered {
        Ok(v) => match json::canonical(&v) {
            Ok(out) => {
                let mut so = std::io::stdout().lock();
                if so.write_all(&out).and_then(|()| so.flush()).is_ok() { 0 } else { 2 }
            }
            Err(t) => {
                eprintln!("gev: {}", t.name());
                2
            }
        },
        Err(m) => {
            eprintln!("gev: {}", m.0);
            2
        }
    };
    std::process::exit(code);
}
