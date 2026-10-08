//! The small dialog for writing or editing a note.

use std::fs;
use std::path::PathBuf;

use gtk::prelude::*;

fn sanitize(title: &str) -> String {
    let t: String = title
        .trim()
        .chars()
        .map(|c| if c == '/' || c.is_control() { '-' } else { c })
        .collect();
    let t = t.trim_start_matches('.').to_string();
    if t.is_empty() { "Note".into() } else { t }
}

pub fn target(title: &str, existing: &Option<PathBuf>) -> PathBuf {
    let dir = crate::notes::dir();
    let base = sanitize(title);
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{base}.txt") } else { format!("{base} {n}.txt") };
        let path = dir.join(name);
        if !path.exists() || existing.as_ref() == Some(&path) {
            return path;
        }
        n += 1;
    }
}

/// Opens the editor. `existing` is the note being edited, or None for a new one.
pub fn open(existing: Option<PathBuf>) {
    let (title, body) = existing
        .as_deref()
        .map(crate::notes::read)
        .unwrap_or_default();

    let dialog = gtk::Dialog::with_buttons(
        Some(if existing.is_some() { "Edit note" } else { "New note" }),
        None::<&gtk::Window>,
        gtk::DialogFlags::empty(),
        &[("Cancel", gtk::ResponseType::Cancel), ("Save", gtk::ResponseType::Accept)],
    );
    dialog.set_default_size(400, 320);
    dialog.set_default_response(gtk::ResponseType::Accept);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Title"));
    entry.set_text(&title);
    let text = gtk::TextView::new();
    text.set_wrap_mode(gtk::WrapMode::WordChar);
    text.set_left_margin(6);
    text.set_top_margin(6);
    text.buffer().expect("text buffer").set_text(&body);
    let scroll = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    scroll.set_vexpand(true);
    scroll.set_shadow_type(gtk::ShadowType::In);
    scroll.add(&text);
    let hint = gtk::Label::new(Some("Start a line with a date for a reminder, e.g.  10/5/31 3:30pm Dentist"));
    hint.set_xalign(0.0);
    hint.set_opacity(0.6);

    let content = dialog.content_area();
    content.set_spacing(8);
    content.set_border_width(10);
    content.pack_start(&entry, false, false, 0);
    content.pack_start(&scroll, true, true, 0);
    content.pack_start(&hint, false, false, 0);

    let entry_in = entry.clone();
    dialog.connect_response(move |d, resp| {
        if resp == gtk::ResponseType::Accept {
            let buf = text.buffer().expect("text buffer");
            let body = buf.text(&buf.start_iter(), &buf.end_iter(), false).map(|s| s.to_string()).unwrap_or_default();
            let dest = target(&entry_in.text(), &existing);
            let _ = fs::create_dir_all(crate::notes::dir());
            if fs::write(&dest, body).is_ok() {
                if let Some(old) = existing.as_ref().filter(|o| **o != dest) {
                    let _ = fs::remove_file(old);
                }
            }
        }
        d.close();
    });
    dialog.show_all();
    entry.grab_focus();
}
