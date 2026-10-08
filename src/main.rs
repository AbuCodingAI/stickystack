//! StickyStack: files on ~/Desktop become glossy fridge magnets, and your own
//! notes become curled sticky notes (with date reminders) on the desktop.

mod editor;
mod magnet;
mod notes;
mod plus;
mod remind;
mod store;
mod win;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use gtk::prelude::*;
use notify::{RecursiveMode, Watcher};

struct State {
    desktop: PathBuf,
    notes_dir: PathBuf,
    /// key -> (window, modification time it was drawn from)
    wins: HashMap<String, (gtk::Window, Option<SystemTime>)>,
    positions: Rc<RefCell<store::Positions>>,
}

fn desktop_dir() -> PathBuf {
    glib::user_special_dir(glib::UserDirectory::Desktop)
        .unwrap_or_else(|| glib::home_dir().join("Desktop"))
}

/// Visible entries of `dir` as (file name, modification time), sorted by name.
fn list(dir: &PathBuf) -> Vec<(String, Option<SystemTime>)> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let mtime = e.metadata().ok()?.modified().ok();
            (!name.starts_with('.')).then_some((name, mtime))
        })
        .collect();
    v.sort();
    v
}

fn sync(state: &Rc<RefCell<State>>) {
    let mut s = state.borrow_mut();
    let mut wanted: HashMap<String, (PathBuf, Option<SystemTime>, bool)> = HashMap::new();
    for (name, m) in list(&s.desktop) {
        wanted.insert(name.clone(), (s.desktop.join(name), m, false));
    }
    for (name, m) in list(&s.notes_dir).into_iter().filter(|(n, _)| n.ends_with(".txt")) {
        wanted.insert(format!("note/{name}"), (s.notes_dir.join(name), m, true));
    }

    let stale: Vec<String> = s
        .wins
        .iter()
        .filter(|(k, (_, m))| wanted.get(*k).map_or(true, |(_, wm, _)| wm != m))
        .map(|(k, _)| k.clone())
        .collect();
    for k in stale {
        if let Some((w, _)) = s.wins.remove(&k) {
            w.close();
        }
    }

    let positions = s.positions.clone();
    let mut keys: Vec<_> = wanted.keys().cloned().collect();
    keys.sort();
    for key in keys {
        if s.wins.contains_key(&key) {
            continue;
        }
        let (path, mtime, is_note) = &wanted[&key];
        let slot = s.wins.len();
        let win = if *is_note {
            notes::create(path, &key, slot, positions.clone())
        } else {
            magnet::create(path, &key, slot, positions.clone())
        };
        s.wins.insert(key, (win, *mtime));
    }
}

fn main() {
    if gtk::init().is_err() {
        eprintln!("stickystack: could not initialise GTK");
        std::process::exit(1);
    }

    let desktop = desktop_dir();
    let notes_dir = notes::dir();
    let _ = fs::create_dir_all(&notes_dir);
    let positions = Rc::new(RefCell::new(store::Positions::load()));
    let state = Rc::new(RefCell::new(State {
        desktop: desktop.clone(),
        notes_dir: notes_dir.clone(),
        wins: HashMap::new(),
        positions: positions.clone(),
    }));
    remind::check(&notes_dir);
    sync(&state);
    let _plus = plus::create(positions);

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })
    .expect("could not create file watcher");
    for dir in [&desktop, &notes_dir] {
        if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
            eprintln!("stickystack: cannot watch {}: {e}", dir.display());
        }
    }

    let st = state.clone();
    glib::timeout_add_local(Duration::from_millis(400), move || {
        let mut dirty = false;
        while rx.try_recv().is_ok() {
            dirty = true;
        }
        if dirty {
            sync(&st);
        }
        glib::ControlFlow::Continue
    });

    // reminders: look for newly due dates every 20 seconds
    let st = state.clone();
    glib::timeout_add_seconds_local(20, move || {
        let fired = remind::check(&st.borrow().notes_dir);
        if !fired.is_empty() {
            {
                let mut s = st.borrow_mut();
                for name in fired {
                    if let Some((w, _)) = s.wins.remove(&format!("note/{name}")) {
                        w.close(); // redrawn with its DUE stamp by sync
                    }
                }
            }
            sync(&st);
        }
        glib::ControlFlow::Continue
    });

    let _keep = watcher;
    gtk::main();
}
