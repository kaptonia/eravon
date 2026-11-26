use std::time::Duration;
use eravon::json::Value;
use eravon_chain::endpoints;
use eravon_chain::rpc::{Endpoint, Trouble};
use eravon_chain::send::{self, Confirm};
use eravon_chain::wire::{self, W};

fn w(s: &str) -> W {
    wire::parse(s.as_bytes()).expect("a wire value")
}

struct Says(&'static str, &'static str);

impl Endpoint for Says {
    fn call(&mut self, _: &str, _: &Value) -> Result<W, Trouble> {
        Ok(w(self.1))
    }
    fn name(&self) -> String {
        self.0.into()
    }
    fn place(&self) -> String {
        self.name()
    }
}

struct Falls(&'static str);

impl Endpoint for Falls {
    fn call(&mut self, _: &str, _: &Value) -> Result<W, Trouble> {
        panic!("this node's worker fell")
    }
    fn name(&self) -> String {
        self.0.into()
    }
    fn place(&self) -> String {
        self.name()
    }
}

#[test]
fn receipts_that_differ_by_block_are_not_included() {
    let (mut a, mut b) = (
        Says("http://five", "{\"status\":\"0x1\",\"blockNumber\":\"0x5\",\"logs\":[]}"),
        Says("http://six", "{\"status\":\"0x1\",\"blockNumber\":\"0x6\",\"logs\":[]}"),
    );
    let mut eps: Vec<&mut dyn Endpoint> = vec![&mut a, &mut b];
    let got = send::confirm_each(&mut eps, &[0xab; 32], Duration::ZERO, &[]);
    assert!(!matches!(got, Confirm::Included { .. }), "{got:?}");
}

#[test]
fn a_node_whose_worker_panics_keeps_its_name() {
    let nodes: Vec<(String, Box<dyn Endpoint + Send>)> = vec![
        ("http://one".into(), Box::new(Says("http://one", "\"0x1\""))),
        ("http://falls".into(), Box::new(Falls("http://falls"))),
        ("http://three".into(), Box::new(Says("http://three", "\"0x1\""))),
    ];
    let got = eravon_chain::endpoints::all_in(endpoints::ask_each(nodes, "eth_chainId", &Value::Arr(Vec::new()), eravon_chain::endpoints::every_answer));
    let names: Vec<&str> = got.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["http://one", "http://falls", "http://three"], "every node named, in order");
    assert!(matches!(got[1].1, Err(Trouble::Transport(_))), "{:?}", got[1].1);
    assert!(got[0].1.is_ok() && got[2].1.is_ok());
}
