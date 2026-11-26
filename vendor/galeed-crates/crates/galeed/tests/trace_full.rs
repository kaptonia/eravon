//! The trace sink at its cap, in a process of its own (the sink is settled once per process): a file holding three
//! bytes under the cap takes one three-byte mark exactly, the next is over, and the cap is said once.

use galeed::marks::{dropped, mark, open_to, take_full, Feature, TRACE_CAP_BYTES};

#[test]
fn the_sink_takes_marks_up_to_its_cap_and_says_once_when_full() {
    let dir = std::env::temp_dir().join(format!("galeed-trace-full-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("trace.log");
    std::fs::File::create(&file).unwrap().set_len(TRACE_CAP_BYTES - 3).unwrap();
    assert!(open_to(file.clone()));
    mark(Feature::H1);
    assert_eq!(std::fs::metadata(&file).unwrap().len(), TRACE_CAP_BYTES, "恰好收:`H1\\n` 三字节落到上限");
    assert_eq!((take_full(), dropped()), (None, 0), "恰到上限不算满");
    mark(Feature::H1);
    assert_eq!(std::fs::metadata(&file).unwrap().len(), TRACE_CAP_BYTES, "多一字节不写");
    assert_eq!(dropped(), 1);
    assert_eq!(take_full(), Some(file.clone()), "满了具名说一次");
    mark(Feature::H2);
    assert_eq!(take_full(), None, "只说一次");
    assert_eq!(dropped(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
