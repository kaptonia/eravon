use crate::house::act::{Act, Said};
use crate::shell::Shell;

mod bo;
mod cands;
mod complaint;
mod core;
mod export;
pub(crate) mod first;
mod goods;
mod inner;
mod orders;
mod paylink;
mod payways;
mod queue;
pub(crate) mod settings;
mod wallet;

pub(crate) use goods::follow;

pub fn run(sh: &mut Shell, a: &Act) -> Said {
    sh.desk.rows_gen += 1;
    let domains: [fn(&mut Shell, &Act) -> Option<Said>; 14] = [core::run, complaint::run, first::run, settings::run, orders::run, goods::run, inner::run, cands::run, payways::run, wallet::run, export::run, queue::run, bo::run, paylink::run];
    for d in domains {
        if let Some(said) = d(sh, a) {
            return said;
        }
    }
    Err(format!("{} 在这一层没有执行处", a.step.name()))
}

pub(crate) fn need_seller(sh: &Shell) -> Result<(), String> {
    if sh.desk.seat() == crate::house::shell::Seat::Seller { Ok(()) } else { Err("买家席没有这一页".to_string()) }
}

pub(crate) fn said_of(s: &Option<crate::house::seller::Said>) -> String {
    s.as_ref().map_or(String::new(), |x| format!("{} · {}", x.name, x.detail))
}
