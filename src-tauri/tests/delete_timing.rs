//! Is deleting a transcript actually slow, or does it only feel slow?
use kiku_lib::history::History;
use std::time::Instant;

#[test]
fn deleting_is_effectively_instant() {
    let dir = std::env::temp_dir().join(format!("kiku-del-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let history = History::open(&dir.join("h.sqlite3")).unwrap();
    for i in 0..2000 {
        history
            .insert(&format!("transcript number {i}"), 1000, 60, Some("m"))
            .unwrap();
    }

    let page = history.list(None, 50, 0).unwrap();
    let id = page.entries[0].id;

    let t = Instant::now();
    history.delete(i64::from(id)).unwrap();
    let micros = t.elapsed().as_micros();

    println!("  delete one row out of 2000: {micros} microseconds");
    assert!(
        micros < 50_000,
        "deletion should not be perceptible: {micros}us"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
