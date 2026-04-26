use std::{collections::HashSet, fs, path::Path, sync::mpsc, time::Duration};

use notify::Result;

use from_do_compiler as compiler;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::Watch;

pub fn watch(args: Watch) -> Result<()> {
    println!("Watching: '{}'", args.path);

    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(100), tx)?;
    debouncer
        .watcher()
        .watch(Path::new(&args.path), notify::RecursiveMode::Recursive)?;

    let mut updated = HashSet::new();

    for result in rx {
        match result {
            Ok(es) => {
                for e in &es {
                    if !matches!(e.kind, DebouncedEventKind::Any) {
                        continue;
                    }
                    if !e.path.is_file() || e.path.extension().unwrap_or_default() != "fromdo" {
                        continue;
                    }
                    println!("Event: {:?}", e);
                    if !updated.remove(&e.path) {
                        println!("update source {:?}", e.path);
                        let source = fs::read_to_string(&e.path).unwrap();
                        match compiler::eval(&source) {
                            Ok(output) => {
                                fs::write(&e.path, output).unwrap();
                                updated.insert(e.path.clone());
                            }
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {:?}", e)
            }
        }
    }

    Ok(())
}
