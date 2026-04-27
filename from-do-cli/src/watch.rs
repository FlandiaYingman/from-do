use std::{collections::HashSet, path::Path, sync::Arc, time::Duration};

use from_do_compiler as compiler;
use notify::Result;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
use tokio::sync::{Mutex, mpsc};

use crate::Watch;

pub async fn watch(args: Watch) -> Result<()> {
    println!("Watching: '{}'", args.path);

    let (tx, mut rx) = mpsc::unbounded_channel::<notify_debouncer_mini::DebouncedEvent>();

    let mut debouncer = new_debouncer(Duration::from_millis(100), move |result| match result {
        Ok(events) => {
            for e in events {
                let _ = tx.send(e);
            }
        }
        Err(e) => eprintln!("watch error: {:?}", e),
    })?;
    debouncer
        .watcher()
        .watch(Path::new(&args.path), notify::RecursiveMode::Recursive)?;

    // suppress duplicate events
    let fs = Arc::new(Mutex::new(HashSet::new()));

    while let Some(event) = rx.recv().await {
        if !matches!(event.kind, DebouncedEventKind::Any) {
            continue;
        }
        if !event.path.is_file() || event.path.extension().unwrap_or_default() != "fromdo" {
            continue;
        }

        let f = Arc::clone(&fs);
        tokio::spawn(async move {
            let path = event.path;

            if f.lock().await.remove(&path) {
                return;
            }

            println!("Compiling: {:?}", path);

            let eval_path = path.clone();
            let eval_future = tokio::task::spawn_blocking(move || -> std::io::Result<_> {
                let source = std::fs::read_to_string(&eval_path)?;
                Ok(compiler::eval(&source))
            })
            .await;

            let r = match eval_future {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    eprintln!("fs error ({:?}): {}", path, e);
                    return;
                }
                Err(e) => {
                    eprintln!("compiler panic ({:?}): {}", path, e);
                    return;
                }
            };

            match r {
                Ok(output) => {
                    f.lock().await.insert(path.clone());
                    let future_path = path.clone();
                    let future =
                        tokio::task::spawn_blocking(move || std::fs::write(&future_path, output))
                            .await;
                    match future {
                        Ok(Ok(())) => {
                            println!("Program Ok {:?}", path)
                        }
                        Ok(Err(e)) => eprintln!("fs error ({:?}): {}", path, e),
                        Err(e) => eprintln!("fs panic ({:?}): {}", path, e),
                    }
                }
                Err(e) => {
                    eprintln!("Program Err ({:?}):", path);
                    for err in e.iter().take(3) {
                        eprintln!("  - {:?}", err);
                    }
                    if e.len() > 3 {
                        eprintln!("  ... and {} more", e.len() - 3);
                    }
                }
            }
        });
    }

    Ok(())
}
