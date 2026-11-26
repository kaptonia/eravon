use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

use crate::fault::Fault;

pub fn entry<R>(run: impl FnOnce() -> R) -> R {
    let _action = crate::action::open(None);
    run()
}

#[derive(Clone, Debug)]
pub enum Outcome<T> {
    Item(T),
    Done,
    Failed(Fault),
}

pub struct Task<T> {
    rx: Receiver<Outcome<T>>,
    handle: Option<std::thread::JoinHandle<()>>,
    pub idle_ticks: u64,
    pub taken: u64,
}

fn cut_short() -> Fault {
    Fault::Usage("后台那一件没答完就断了(它的线程中途退出)".into())
}

pub fn spawn<T, F>(name: &str, f: F) -> Task<T>
where
    T: Send + 'static,
    F: FnOnce(&Sender<Outcome<T>>) + Send + 'static,
{
    let (tx, rx) = channel();
    let refused = tx.clone();
    let handle = match eravon_os::start_named(name, move || f(&tx)) {
        Ok(h) => Some(h),
        Err(e) => {
            let _ = refused.send(Outcome::Failed(Fault::Transport(e.to_string())));
            None
        }
    };
    Task { rx, handle, idle_ticks: 0, taken: 0 }
}


impl<T> Task<T> {
    pub fn take(&mut self) -> Vec<Outcome<T>> {
        let mut out = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(o) => {
                    if self.push(o, &mut out) {
                        return out;
                    }
                }
                Err(TryRecvError::Empty) => {
                    if out.is_empty() {
                        self.idle_ticks += 1;
                    }
                    return out;
                }
                Err(TryRecvError::Disconnected) => {
                    out.push(Outcome::Failed(cut_short()));
                    return out;
                }
            }
        }
    }

    pub fn wait(&mut self, budget: Duration) -> Vec<Outcome<T>> {
        let deadline = Instant::now() + budget;
        let mut out = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(o) => {
                    if self.push(o, &mut out) {
                        return out;
                    }
                }
                Err(RecvTimeoutError::Timeout) => return out,
                Err(RecvTimeoutError::Disconnected) => {
                    out.push(Outcome::Failed(cut_short()));
                    return out;
                }
            }
        }
    }

    pub fn take_handle(&mut self) -> Option<std::thread::JoinHandle<()>> {
        self.handle.take()
    }

    fn push(&mut self, o: Outcome<T>, out: &mut Vec<Outcome<T>>) -> bool {
        self.taken += 1;
        let end = matches!(o, Outcome::Done | Outcome::Failed(_));
        out.push(o);
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_gone_without_its_last_word_ends_by_name() {
        let mut t: Task<u8> = spawn("test", |tx| {
            let _ = tx.send(Outcome::Item(1));
        });
        let got = t.wait(Duration::from_secs(5));
        assert!(matches!(got.last(), Some(Outcome::Failed(crate::fault::Fault::Usage(_)))));
    }

    #[test]
    fn the_worker_thread_is_named_after_its_kind() {
        let mut t: Task<String> = spawn("orders", |tx| {
            let _ = tx.send(Outcome::Item(std::thread::current().name().unwrap_or_default().to_string()));
            let _ = tx.send(Outcome::Done);
        });
        let got = t.wait(Duration::from_secs(5));
        assert!(matches!(got.first(), Some(Outcome::Item(n)) if n == "eravon-orders"), "{got:?}");
    }

    #[test]
    fn a_thread_the_system_will_not_start_is_a_named_end() {
        eravon_os::refuse_threads(true);
        let mut t: Task<u8> = spawn("refused", |_| {});
        eravon_os::refuse_threads(false);
        let got = t.take();
        assert!(matches!(got.as_slice(), [Outcome::Failed(f)] if f.name() == "E_CHAIN_TRANSPORT" && f.detail().starts_with("thread ")), "{got:?}");
    }

    #[test]
    fn taking_never_waits() {
        let (go_tx, go_rx) = channel::<()>();
        let mut t: Task<u8> = spawn("idle", move |tx| {
            let _ = go_rx.recv();
            let _ = tx.send(Outcome::Done);
        });
        let at = Instant::now();
        assert!(t.take().is_empty());
        assert!(at.elapsed() < Duration::from_millis(50) && t.idle_ticks == 1);
        go_tx.send(()).unwrap();
        assert!(matches!(t.wait(Duration::from_secs(5)).as_slice(), [Outcome::Done]));
    }
}
