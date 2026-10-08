//! Your own notes: plain `.txt` files in ~/.local/share/stickystack/notes,
//! drawn as curled sticky notes. Right-click to edit or send to the Trash.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;

use crate::store::Positions;
use crate::win;

const PAD: f64 = 14.0; // room around the paper for the drop shadow
const SIZE: f64 = 190.0; // paper edge
const CURL: f64 = 40.0; // size of the curled corner

const PALETTE: [(f64, f64, f64); 5] = [
    (1.00, 0.93, 0.45), // classic yellow
    (1.00, 0.76, 0.82), // pink
    (0.69, 0.92, 0.78), // mint
    (0.68, 0.84, 1.00), // sky
    (1.00, 0.82, 0.60), // orange
];

pub fn dir() -> PathBuf {
    glib::user_data_dir().join("stickystack").join("notes")
}

/// (title, body) of a note file. The title is the file name.
pub fn read(path: &Path) -> (String, String) {
    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    (title, std::fs::read_to_string(path).unwrap_or_default())
}

fn shade((r, g, b): (f64, f64, f64), k: f64) -> (f64, f64, f64) {
    ((r * k).min(1.0), (g * k).min(1.0), (b * k).min(1.0))
}

fn layout(cr: &cairo::Context, font: &str, width: f64, lines: i32) -> pango::Layout {
    let l = pangocairo::functions::create_layout(cr);
    l.set_font_description(Some(&pango::FontDescription::from_string(font)));
    l.set_width((width * pango::SCALE as f64) as i32);
    l.set_wrap(pango::WrapMode::WordChar);
    l.set_ellipsize(pango::EllipsizeMode::End);
    l.set_height(-lines);
    l
}

/// The paper outline: a square whose bottom-right corner is cut away.
fn paper_path(cr: &cairo::Context, x: f64, y: f64) {
    let (x1, y1) = (x + SIZE, y + SIZE);
    cr.new_path();
    cr.move_to(x, y);
    cr.line_to(x1, y);
    cr.line_to(x1, y1 - CURL);
    cr.curve_to(x1 - CURL * 0.2, y1 - CURL * 0.8, x1 - CURL * 0.8, y1 - CURL * 0.2, x1 - CURL, y1);
    cr.line_to(x, y1);
    cr.close_path();
}

fn draw(cr: &cairo::Context, fx: crate::win::Fx, colour: (f64, f64, f64), title: &str, body: &str, due: bool) {
    let (x, y) = (PAD, PAD);
    let (x1, y1) = (x + SIZE, y + SIZE);

    cr.set_operator(cairo::Operator::Clear);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);

    // soft drop shadow: stacked translucent copies of the outline
    for i in 0..9 {
        let grow = i as f64;
        cr.save().ok();
        cr.translate(2.0 + grow * 0.2, 5.0 + grow * 0.4 + (fx.lift - fx.press * 0.6) * 3.0);
        paper_path(cr, x - grow * 0.5, y - grow * 0.5);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.035);
        let _ = cr.fill();
        cr.restore().ok();
    }
    // the note lifts off the wall on hover and settles back when pressed
    cr.translate(0.0, fx.dy(4.0));

    // paper, lit from the top-left
    paper_path(cr, x, y);
    let g = cairo::LinearGradient::new(x, y, x1, y1);
    let (r, gg, b) = shade(colour, 1.03);
    g.add_color_stop_rgb(0.0, r, gg, b);
    let (r, gg, b) = shade(colour, 0.93);
    g.add_color_stop_rgb(1.0, r, gg, b);
    let _ = cr.set_source(&g);
    let _ = cr.fill_preserve();
    cr.clip();

    // adhesive strip along the top
    let strip = cairo::LinearGradient::new(0.0, y, 0.0, y + 22.0);
    strip.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.07);
    strip.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
    let _ = cr.set_source(&strip);
    cr.rectangle(x, y, SIZE, 22.0);
    let _ = cr.fill();

    // text
    let ink = (0.18, 0.14, 0.10);
    let inner = SIZE - 28.0;
    cr.set_source_rgb(ink.0, ink.1, ink.2);
    cr.move_to(x + 14.0, y + 16.0);
    let t = layout(cr, "Caveat, Comic Neue, Comic Sans MS, Sans Bold 17", inner, 2);
    t.set_text(title);
    pangocairo::functions::show_layout(cr, &t);
    let (_, th) = t.pixel_size();

    cr.set_source_rgba(ink.0, ink.1, ink.2, 0.25);
    cr.set_line_width(1.0);
    cr.move_to(x + 14.0, y + 22.0 + th as f64);
    cr.line_to(x1 - 14.0, y + 22.0 + th as f64);
    let _ = cr.stroke();

    cr.set_source_rgba(ink.0, ink.1, ink.2, 0.85);
    cr.move_to(x + 14.0, y + 30.0 + th as f64);
    let l = layout(cr, "Caveat, Comic Neue, Comic Sans MS, Sans 14", inner, 6);
    l.set_text(body.trim());
    pangocairo::functions::show_layout(cr, &l);
    cr.reset_clip();

    // shadow the lifted flap casts onto the paper
    let (ax, ay) = (x1, y1 - CURL);
    let (bx, by) = (x1 - CURL, y1);
    cr.save().ok();
    paper_path(cr, x, y);
    cr.clip();
    let sh = cairo::LinearGradient::new(x1, y1, x1 - CURL * 0.55, y1 - CURL * 0.55);
    sh.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.0);
    sh.add_color_stop_rgba(0.45, 0.0, 0.0, 0.0, 0.0);
    sh.add_color_stop_rgba(0.62, 0.0, 0.0, 0.0, 0.22);
    sh.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 0.0);
    let _ = cr.set_source(&sh);
    cr.move_to(ax + 4.0, ay - 6.0);
    cr.line_to(bx - 6.0, by + 4.0);
    cr.line_to(x1 + 6.0, y1 + 6.0);
    cr.close_path();
    let _ = cr.fill();
    cr.restore().ok();

    // the curled flap: the corner folded back over itself, rounded like paper
    let (px, py) = (x1 - CURL, y1 - CURL);
    cr.new_path();
    cr.move_to(ax, ay);
    cr.curve_to(ax - CURL * 0.05, py + CURL * 0.35, px + CURL * 0.25, py + CURL * 0.05, px + 1.0, py + 1.0);
    cr.curve_to(px + CURL * 0.1, py + CURL * 0.4, bx, by - CURL * 0.25, bx, by);
    cr.curve_to(x1 - CURL * 0.8, y1 - CURL * 0.2, x1 - CURL * 0.2, y1 - CURL * 0.8, ax, ay);
    cr.close_path();
    let flap = cairo::LinearGradient::new(px, py, x1 - CURL * 0.5, y1 - CURL * 0.5);
    let (r, gg, b) = shade(colour, 1.08);
    flap.add_color_stop_rgb(0.0, r, gg, b);
    let (r, gg, b) = shade(colour, 0.80);
    flap.add_color_stop_rgb(1.0, r, gg, b);
    let _ = cr.set_source(&flap);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.10);
    cr.set_line_width(0.6);
    let _ = cr.stroke();

    // a red stamp once a reminder on this note has come due
    if due {
        cr.save().ok();
        cr.translate(x1 - 52.0, y + 6.0);
        cr.rotate(0.21);
        cr.set_source_rgba(0.78, 0.10, 0.10, 0.85);
        let l = layout(cr, "Sans Bold 11", 60.0, 1);
        l.set_text("DUE");
        pangocairo::functions::show_layout(cr, &l);
        cr.restore().ok();
    }
}

pub fn create(path: &Path, key: &str, slot: usize, positions: Rc<RefCell<Positions>>) -> gtk::Window {
    let (title, body) = read(path);
    let h = title.bytes().fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    let colour = PALETTE[h as usize % PALETTE.len()];
    let due = crate::remind::any_due(&body);

    let menu = gtk::Menu::new();
    let p = path.to_path_buf();
    win::item(&menu, "Edit", move || crate::editor::open(Some(p.clone())));
    let (t, b) = (title.clone(), body.clone());
    win::item(&menu, "Duplicate", move || {
        let dest = crate::editor::target(&format!("{t} copy"), &None);
        let _ = std::fs::write(dest, &b);
    });
    let b = body.clone();
    win::item(&menu, "Copy text", move || win::copy_text(&b));
    win::separator(&menu);
    let p = path.to_path_buf();
    win::item(&menu, "Move to Trash", move || win::trash(&p));
    menu.show_all();

    let p = path.to_path_buf();
    let t = title.clone();
    win::build(win::Spec {
        key,
        title: &title,
        tooltip: None,
        size: (SIZE + PAD * 2.0) as i32,
        slot,
        origin: None,
        single_click: false,
        positions,
        draw: Box::new(move |cr, fx| draw(cr, fx, colour, &t, &body, due)),
        on_open: Box::new(move || crate::editor::open(Some(p.clone()))),
        menu,
    })
}
