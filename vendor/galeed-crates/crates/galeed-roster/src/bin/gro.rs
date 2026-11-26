//! `gro`:K4 的夹具二进制,十二命令照 HARNESS-ROSTER.md。stdout 恰一个正典 JSON 值、无尾换行;答出 0,误用 2。
//! 形闸与答案都在库里(`galeed_roster::harness`),这里只起进程、写出、给退出码。
#![forbid(unsafe_code)]

use galeed::json;
use std::io::Write;

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let code = match galeed_roster::harness::run(&args) {
        Ok(v) => match json::canonical(&v) {
            Ok(out) => {
                let mut so = std::io::stdout().lock();
                if so.write_all(&out).and_then(|()| so.flush()).is_ok() { 0 } else { 2 }
            }
            Err(t) => {
                eprintln!("gro: {}", t.name());
                2
            }
        },
        Err(m) => {
            eprintln!("gro: {}", m.0);
            2
        }
    };
    std::process::exit(code);
}
