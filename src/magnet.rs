//! One fridge magnet: a borderless, translucent window drawn with Cairo.
//! A thick glossy black slab with the file's icon sitting on its face, a
//! bevelled rim, a domed highlight and a real shadow on the wall.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::prelude::*;

use crate::store::Positions;
use crate::win::{self, Fx};

const PAD: f64 = 14.0; // room around the magnet for its shadow
const SIZE: f64 = 56.0; // face edge
const THICK: f64 = 8.0; // how far the magnet stands off the surface
const ICON: i32 = 40;
const RADIUS: f64 = 14.0;

type Pixbuf = gdk::gdk_pixbuf::Pixbuf;

/// The launcher entry behind a `.desktop` file, if that's what this is.
fn desktop_entry(path: &Path) -> Option<gio::DesktopAppInfo> {
    if path.extension()? != "desktop" {
        return None;
    }
    gio::DesktopAppInfo::from_filename(path)
}

/// The icon a file manager would show: a launcher's own `Icon=`, otherwise
/// the one for the file's type.
fn icon_for(path: &Path) -> Option<Pixbuf> {
    let icon = match desktop_entry(path).and_then(|app| app.icon()) {
        Some(icon) => icon,
        None => {
            let ctype = if path.is_dir() {
                "inode/directory".into()
            } else {
                gio::content_type_guess(Some(path), &[]).0
            };
            gio::content_type_get_icon(&ctype)
        }
    };
    gtk::IconTheme::default()?
        .lookup_by_gicon(&icon, ICON, gtk::IconLookupFlags::FORCE_SIZE)?
        .load_icon()
        .ok()
}

fn slab_path(cr: &cairo::Context, x: f64, y: f64, s: f64) {
    let r = RADIUS;
    cr.new_path();
    cr.arc(x + s - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + s - r, y + s - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + s - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}

fn draw(cr: &cairo::Context, fx: Fx, icon: &Option<Pixbuf>) {
    let (x, y, s) = (PAD, PAD, SIZE);
    let k = SIZE / 104.0; // effects were tuned at 104px; scale them with the size

    cr.set_operator(cairo::Operator::Clear);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);

    // shadow cast on the fridge: wide and soft, plus a tight dark contact edge;
    // it spreads and softens as the magnet is lifted, and tightens when pressed
    let lift = fx.lift - fx.press * 0.6;
    for i in 0..14 {
        let g = i as f64;
        slab_path(cr, x + (5.0 - g * 0.5) * k, y + THICK + (6.0 + lift * 5.0 - g * 0.4) * k, s + g * k);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.025);
        let _ = cr.fill();
    }
    slab_path(cr, x + 1.5 * k, y + THICK + (2.0 + lift * 2.0) * k, s);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.45 - lift * 0.15);
    let _ = cr.fill();

    // the magnet itself rises toward you on hover and sinks when pressed
    cr.translate(0.0, fx.dy(5.0 * k));

    // the slab's side wall: stacked copies of the outline, from the bottom up,
    // so it reads as a solid block that gets darker toward the wall
    let steps = THICK as i32 * 2;
    for i in (0..=steps).rev() {
        let off = i as f64 * THICK / steps as f64;
        slab_path(cr, x, y + off, s);
        let v = 0.17 * (1.0 - off / THICK) + 0.012;
        cr.set_source_rgb(v, v, v * 1.05);
        let _ = cr.fill();
    }
    // a faint rim light along the bottom edge of the slab
    slab_path(cr, x + 1.5, y + THICK - 0.5, s - 3.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
    cr.set_line_width(1.0);
    let _ = cr.stroke();

    // glossy black face, lit from the top-left
    slab_path(cr, x, y, s);
    let face = cairo::RadialGradient::new(x + s * 0.3, y + s * 0.22, 4.0, x + s * 0.5, y + s * 0.5, s * 0.85);
    face.add_color_stop_rgb(0.0, 0.25, 0.25, 0.27);
    face.add_color_stop_rgb(0.5, 0.11, 0.11, 0.12);
    face.add_color_stop_rgb(1.0, 0.03, 0.03, 0.035);
    let _ = cr.set_source(&face);
    let _ = cr.fill_preserve();
    cr.clip_preserve();

    // bevelled rim: light edge top-left, deep edge bottom-right
    let rim = cairo::LinearGradient::new(x, y, x + s, y + s);
    rim.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.45);
    rim.add_color_stop_rgba(0.4, 1.0, 1.0, 1.0, 0.0);
    rim.add_color_stop_rgba(0.6, 0.0, 0.0, 0.0, 0.0);
    rim.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.6);
    let _ = cr.set_source(&rim);
    cr.set_line_width(5.0 * k);
    let _ = cr.stroke();

    // the icon alone, centred on the face (the name appears as a tooltip)
    if let Some(pb) = icon {
        let (w, h) = (pb.width() as f64, pb.height() as f64);
        cr.set_source_pixbuf(pb, x + (s - w) / 2.0, y + (s - h) / 2.0);
        let _ = cr.paint();
    }

    // domed gloss across the upper half, over everything
    cr.new_path();
    cr.move_to(x - 4.0, y - 4.0);
    cr.line_to(x + s + 4.0, y - 4.0);
    cr.line_to(x + s + 4.0, y + s * 0.34);
    cr.curve_to(x + s * 0.72, y + s * 0.52, x + s * 0.28, y + s * 0.52, x - 4.0, y + s * 0.30);
    cr.close_path();
    let gloss = cairo::LinearGradient::new(x, y, x, y + s * 0.5);
    gloss.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.22);
    gloss.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.02);
    let _ = cr.set_source(&gloss);
    let _ = cr.fill();
    cr.reset_clip();

    // thin specular glint hugging the top-left rim
    slab_path(cr, x + 2.0, y + 2.0, s - 4.0);
    let glint = cairo::LinearGradient::new(x, y, x + s * 0.6, y + s * 0.6);
    glint.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.7);
    glint.add_color_stop_rgba(0.5, 1.0, 1.0, 1.0, 0.0);
    let _ = cr.set_source(&glint);
    cr.set_line_width(1.0);
    let _ = cr.stroke();
}

/// Launchers start the app; everything else opens with the default program.
fn open(path: &Path) {
    if let Some(app) = desktop_entry(path) {
        if app.launch(&[], gio::AppLaunchContext::NONE).is_ok() {
            return;
        }
    }
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

fn reveal(path: &Path) {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"));
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
}

pub fn create(path: &Path, name: &str, slot: usize, positions: Rc<RefCell<Positions>>) -> gtk::Window {
    let icon = icon_for(path);
    let label = desktop_entry(path)
        .map(|app| app.name().to_string())
        .unwrap_or_else(|| name.to_string());

    let menu = gtk::Menu::new();
    let p = path.to_path_buf();
    win::item(&menu, "Open", move || open(&p));
    let p = path.to_path_buf();
    win::item(&menu, "Show in folder", move || reveal(&p));
    let p = path.to_path_buf();
    win::item(&menu, "Copy path", move || win::copy_text(&p.to_string_lossy()));
    win::separator(&menu);
    let p = path.to_path_buf();
    win::item(&menu, "Move to Trash", move || win::trash(&p));
    menu.show_all();

    let p = path.to_path_buf();
    win::build(win::Spec {
        key: name,
        title: name,
        tooltip: Some(label),
        size: (SIZE + THICK + PAD * 2.0) as i32,
        slot,
        origin: None,
        single_click: false,
        positions,
        draw: Box::new(move |cr, fx| draw(cr, fx, &icon)),
        on_open: Box::new(move || open(&p)),
        menu,
    })
}
