//! A trace sink that cannot be written, in a process of its own: marks still land in the ring, the trouble is
//! handed out once with the system's words, and nothing stops.

use galeed::marks::{dropped, mark, open_to, ring, take_trouble, Feature};

#[test]
fn a_sink_that_cannot_be_written_is_said_once_and_stops_nothing() {
    let at = std::env::temp_dir().join(format!("galeed-trace-missing-{}", std::process::id())).join("no-such-folder").join("trace.log");
    assert!(open_to(at.clone()));
    mark(Feature::G1);
    mark(Feature::G2);
    assert_eq!(ring(), vec!["G1", "G2"], "环照收");
    assert_eq!(dropped(), 2);
    let (path, said) = take_trouble().expect("写不出要说");
    assert_eq!(path, at);
    assert!(!said.is_empty(), "带系统原话");
    assert_eq!(take_trouble(), None, "只说一次");
}
