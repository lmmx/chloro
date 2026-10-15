//! The unit cache gives the sequential formatter's output, cold and warm.
//!
//! `format_source_with_unit_cache` stitches units it formatted and units it found in the
//! cache, rebased to where they now are. This test holds it to `format_source_with_config`
//! on every eleventh conformance fixture, under the default options and with the reordering
//! of imports and modules off, at one and four threads: a cold pass (every unit formatted and
//! stored), a warm pass (every unit found), and the file moved down by a comment line (every
//! position a hit replays is rebased).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;

use chloro_core::formatter::config::Config;
use chloro_core::{
    CachedUnit, UnitCache, UnitKey, format_source_with_config, format_source_with_unit_cache,
};

#[derive(Default)]
struct Exact {
    map: RefCell<HashMap<(Vec<u8>, String), CachedUnit>>,
    lookups: Cell<usize>,
    stores: Cell<usize>,
}

impl UnitCache for Exact {
    fn get(&self, key: &UnitKey<'_>) -> Option<CachedUnit> {
        self.lookups.set(self.lookups.get() + 1);
        self.map
            .borrow()
            .get(&(key.context().to_vec(), key.text().to_owned()))
            .cloned()
    }

    fn insert(&self, key: &UnitKey<'_>, unit: CachedUnit) {
        self.stores.set(self.stores.get() + 1);
        self.map
            .borrow_mut()
            .insert((key.context().to_vec(), key.text().to_owned()), unit);
    }
}

#[test]
fn unit_cache_matches_the_sequential_formatter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/fixtures");
    let mut files: Vec<_> = walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    assert!(files.len() > 1000, "{} fixtures found", files.len());

    let mut no_reorder = Config::default();
    no_reorder.set("reorder_imports", "false").unwrap();
    no_reorder.set("reorder_modules", "false").unwrap();
    let configs = [
        ("default", Config::default()),
        ("no reordering", no_reorder),
    ];

    let (mut checked, mut warm_units) = (0, 0);
    for path in files.iter().step_by(11) {
        let source = std::fs::read_to_string(path).unwrap();
        let name_of = || path.strip_prefix(&root).unwrap().display().to_string();
        for (name, config) in &configs {
            let sequential = format_source_with_config(&source, config);
            for threads in [1, 4] {
                let cache = Exact::default();
                let cold = format_source_with_unit_cache(&source, config, threads, &cache);
                assert!(
                    cold == sequential,
                    "{} cold, {threads} threads ({name})",
                    name_of()
                );
                let lookups = cache.lookups.take();
                cache.stores.take();
                let warm = format_source_with_unit_cache(&source, config, threads, &cache);
                assert!(
                    warm == sequential,
                    "{} warm, {threads} threads ({name})",
                    name_of()
                );
                assert_eq!(cache.lookups.take(), lookups, "{}", name_of());
                // A warm pass formats (and so stores) no unit.
                assert_eq!(cache.stores.take(), 0, "{} warm stores", name_of());
                warm_units += lookups;
                if !source.starts_with("#!") && !source.starts_with('\u{feff}') {
                    let moved = format!("// moved\n{source}");
                    assert!(
                        format_source_with_unit_cache(&moved, config, threads, &cache)
                            == format_source_with_config(&moved, config),
                        "{} moved, {threads} threads ({name})",
                        name_of()
                    );
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 100, "{checked} files checked");
    assert!(warm_units > 1000, "{warm_units} units found warm");
}
