//! The trace channel with its environment variable unset, in a process of its own: no sink, no file, nothing to say,
//! marks still kept in the ring.

use galeed::marks::{dropped, mark, ring, sink, take_full, take_trouble, Feature, ENV};

#[test]
fn unset_the_channel_writes_nowhere_and_says_nothing() {
    if std::env::var_os(ENV).is_some_and(|v| !v.is_empty()) {
        panic!("{ENV} 设着跑这一测,测的就不是「未设」");
    }
    mark(Feature::A1);
    assert_eq!(sink(), None);
    assert_eq!(ring(), vec!["A1"]);
    assert_eq!((dropped(), take_trouble(), take_full()), (0, None, None));
}
