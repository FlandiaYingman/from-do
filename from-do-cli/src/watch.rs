use std::{path::Path, sync::mpsc};

use notify::{Event, Result, Watcher};

use crate::Watch;

pub fn watch(args: Watch) -> Result<()> {
    println!("Watching: '{}'", args.path);

    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(Path::new(&args.path), notify::RecursiveMode::Recursive)?;

    for result in rx {
        match result {
            Ok(e) => {
                println!("Event: {:?}", e);
            }
            Err(e) => {
                println!("Error: {:?}", e)
            }
        }
    }

    Ok(())
}
