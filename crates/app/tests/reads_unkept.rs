use std::sync::Arc;

use app::fault::Known;
use eravon_chain::settled::{Keep, Lack, Unkept};

struct Refusing(Unkept);

impl Keep for Refusing {
    fn load(&self, _part: &str) -> Option<Vec<u8>> {
        None
    }
    fn save(&self, _part: &str, _plain: &[u8]) -> Result<(), Unkept> {
        Err(self.0.clone())
    }
}

fn settle_through(keeper: Refusing) {
    let _a = eravon_chain::action::open(Some(Arc::new(keeper)));
    eravon_chain::settled::keep_lack("http://127.0.0.1:1", "eth_getRawTransactionByHash", Lack { said: "method not found".into(), at: 1 }, 0);
}

fn told(shell: &app::shell::Shell) -> Vec<String> {
    shell.faults.iter().filter(|f| f.which() == Some(Known::ReadsUnkept)).map(|f| f.tail().to_string()).collect()
}

#[test]
fn a_part_not_kept_is_told_once_and_one_locked_away_is_not() {
    let ctx = eravon_ui::egui::Context::default();
    let mut shell = app::shell::Shell::boot(eravon_ui::skin::dress(&ctx));
    let before = told(&shell).len();
    settle_through(Refusing(Unkept::Said("No space left on device (os error 28)".into())));
    let _ = shell.drain_at(0.0);
    let now = told(&shell);
    assert_eq!(now.len(), before + 1, "said once: {now:?}");
    assert!(now.last().is_some_and(|t| t.contains("notes") && t.contains("No space left")), "the part and why: {now:?}");
    let _ = shell.drain_at(0.0);
    assert_eq!(told(&shell).len(), before + 1, "said once, not at every drain");
    settle_through(Refusing(Unkept::Locked));
    let _ = shell.drain_at(0.0);
    assert_eq!(told(&shell).len(), before + 1, "the store locked: nothing said");
}
