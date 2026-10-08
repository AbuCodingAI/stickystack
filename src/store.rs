//! Remembers where each note was dragged, in ~/.config/stickystack/positions.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Default)]
pub struct Positions {
    map: HashMap<String, (i32, i32)>,
}

fn file() -> PathBuf {
    glib::user_config_dir().join("stickystack").join("positions")
}

impl Positions {
    pub fn load() -> Self {
        let mut map = HashMap::new();
        if let Ok(text) = fs::read_to_string(file()) {
            for line in text.lines() {
                let mut it = line.rsplitn(3, '\t');
                if let (Some(y), Some(x), Some(name)) = (it.next(), it.next(), it.next()) {
                    if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                        map.insert(name.to_string(), (x, y));
                    }
                }
            }
        }
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<(i32, i32)> {
        self.map.get(name).copied()
    }

    pub fn set(&mut self, name: &str, pos: (i32, i32)) {
        if self.map.insert(name.to_string(), pos) != Some(pos) {
            self.save();
        }
    }

    fn save(&self) {
        let path = file();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let mut out = String::new();
        for (name, (x, y)) in &self.map {
            out.push_str(&format!("{name}\t{x}\t{y}\n"));
        }
        let _ = fs::write(path, out);
    }
}
