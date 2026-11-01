//! Replays every input in `fuzz/corpus/<check>/` through its check, so that inputs the fuzzers
//! found (and the seeds they started from) stay checked on the stable toolchain.

use std::path::Path;

use finalytics_fuzz_checks::CHECKS;

#[test]
fn the_corpus_passes_every_check() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/corpus");
    let mut replayed = 0;
    for (name, check) in CHECKS {
        let Ok(entries) = std::fs::read_dir(root.join(name)) else {
            continue;
        };
        for entry in entries.flatten() {
            let data = std::fs::read(entry.path()).unwrap_or_default();
            check(&data);
            replayed += 1;
        }
    }
    assert!(replayed > 0, "no corpus found under {}", root.display());
}
