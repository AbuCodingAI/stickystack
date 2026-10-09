//! Your own notes: plain `.txt` files in ~/.local/share/stickystack/notes,
//! drawn as curled sticky notes. Right-click to edit or send to the Trash.

use std::cell::RefCell;
use std::f64::consts::PI;
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

/// The paper outline: a square with its bottom-right corner cut away by `cb`
/// and, while peeling from the top, its top-right corner cut away by `ct`.
fn paper_path(cr: &cairo::Context, x: f64, y: f64, cb: f64, ct: f64) {
    let (x1, y1) = (x + SIZE, y + SIZE);
    cr.new_path();
    cr.move_to(x, y);
    if ct > 0.5 {
        cr.line_to(x1 - ct, y);
        cr.curve_to(x1 - ct * 0.8, y + ct * 0.2, x1 - ct * 0.2, y + ct * 0.8, x1, y + ct);
    } else {
        cr.line_to(x1, y);
    }
    cr.line_to(x1, y1 - cb);
    cr.curve_to(x1 - cb * 0.2, y1 - cb * 0.8, x1 - cb * 0.8, y1 - cb * 0.2, x1 - cb, y1);
    cr.line_to(x, y1);
    cr.close_path();
}

/// Title, rule and body text on the paper.
fn text(cr: &cairo::Context, x: f64, y: f64, title: &str, body: &str, alpha: f64) {
    let x1 = x + SIZE;
    let ink = (0.18, 0.14, 0.10);
    let inner = SIZE - 28.0;
    cr.set_source_rgba(ink.0, ink.1, ink.2, alpha);
    cr.move_to(x + 14.0, y + 16.0);
    let t = layout(cr, "Caveat, Comic Neue, Comic Sans MS, Sans Bold 17", inner, 2);
    t.set_text(title);
    pangocairo::functions::show_layout(cr, &t);
    let (_, th) = t.pixel_size();

    cr.set_source_rgba(ink.0, ink.1, ink.2, 0.25 * alpha);
    cr.set_line_width(1.0);
    cr.move_to(x + 14.0, y + 22.0 + th as f64);
    cr.line_to(x1 - 14.0, y + 22.0 + th as f64);
    let _ = cr.stroke();

    cr.set_source_rgba(ink.0, ink.1, ink.2, 0.85 * alpha);
    cr.move_to(x + 14.0, y + 30.0 + th as f64);
    let l = layout(cr, "Caveat, Comic Neue, Comic Sans MS, Sans 14", inner, 6);
    l.set_text(body.trim());
    pangocairo::functions::show_layout(cr, &l);
}

/// A lifted corner: the shadow it casts and the paper folded back over itself.
/// Always drawn for the bottom-right corner; the top one is drawn mirrored.
fn flap(cr: &cairo::Context, x: f64, y: f64, c: f64, colour: (f64, f64, f64)) {
    let (x1, y1) = (x + SIZE, y + SIZE);
    let (ax, ay) = (x1, y1 - c);
    let (bx, by) = (x1 - c, y1);

    cr.save().ok();
    paper_path(cr, x, y, c, 0.0);
    cr.clip();
    let sh = cairo::LinearGradient::new(x1, y1, x1 - c * 0.55, y1 - c * 0.55);
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

    let (px, py) = (x1 - c, y1 - c);
    cr.new_path();
    cr.move_to(ax, ay);
    cr.curve_to(ax - c * 0.05, py + c * 0.35, px + c * 0.25, py + c * 0.05, px + 1.0, py + 1.0);
    cr.curve_to(px + c * 0.1, py + c * 0.4, bx, by - c * 0.25, bx, by);
    cr.curve_to(x1 - c * 0.8, y1 - c * 0.2, x1 - c * 0.2, y1 - c * 0.8, ax, ay);
    cr.close_path();
    let fl = cairo::LinearGradient::new(px, py, x1 - c * 0.5, y1 - c * 0.5);
    let (r, g, b) = shade(colour, 1.08);
    fl.add_color_stop_rgb(0.0, r, g, b);
    let (r, g, b) = shade(colour, 0.80);
    fl.add_color_stop_rgb(1.0, r, g, b);
    let _ = cr.set_source(&fl);
    let _ = cr.fill_preserve();
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.10);
    cr.set_line_width(0.6);
    let _ = cr.stroke();
}

/// The note balled up: its outline slides from the square to a lumpy ball
/// while the paper shrinks, twists, gains creases and the writing fades.
fn crumpled(cr: &cairo::Context, amount: f64, colour: (f64, f64, f64), title: &str, body: &str) {
    let (x, y) = (PAD, PAD);
    let (cx, cy) = (x + SIZE / 2.0, y + SIZE / 2.0);
    let e = amount * amount * (3.0 - 2.0 * amount);
    let half = SIZE / 2.0;

    cr.save().ok();
    cr.translate(cx, cy + e * 10.0);
    cr.rotate(e * 0.7);
    let s = 1.0 - 0.5 * e;
    cr.scale(s, s);
    cr.translate(-cx, -cy);

    let n = 40;
    let outline = |cr: &cairo::Context, dx: f64, dy: f64| {
        cr.new_path();
        for i in 0..n {
            let t = i as f64 / n as f64 * 2.0 * PI;
            let rect = half / t.cos().abs().max(t.sin().abs());
            let lump = 0.11 * (3.0 * t + 1.0).sin() + 0.07 * (5.0 * t + 2.0).sin() + 0.04 * (7.0 * t).sin();
            let ball = half * 0.82 * (1.0 + lump);
            let r = rect + (ball - rect) * e;
            let (px, py) = (cx + dx + r * t.cos(), cy + dy + r * t.sin());
            if i == 0 { cr.move_to(px, py) } else { cr.line_to(px, py) }
        }
        cr.close_path();
    };

    for i in 0..6 {
        outline(cr, 2.0 + i as f64, 5.0 + i as f64 * 1.5);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.04);
        let _ = cr.fill();
    }

    outline(cr, 0.0, 0.0);
    let g = cairo::RadialGradient::new(cx - half * 0.4, cy - half * 0.4, 4.0, cx, cy, half * 1.3);
    let (r, gg, b) = shade(colour, 1.08 - 0.05 * e);
    g.add_color_stop_rgb(0.0, r, gg, b);
    let (r, gg, b) = shade(colour, 0.88 - 0.12 * e);
    g.add_color_stop_rgb(1.0, r, gg, b);
    let _ = cr.set_source(&g);
    let _ = cr.fill_preserve();
    cr.clip();

    text(cr, x, y, title, body, (1.0 - e * 2.2).max(0.0));

    // creases: alternating dark ridges and light edges, deepening as it balls up
    for j in 0..18 {
        let a1 = j as f64 * 0.97;
        let a2 = a1 + 1.6 + (j % 4) as f64 * 0.45;
        let (r1, r2) = (half * (0.15 + (j % 5) as f64 * 0.17), half * (0.25 + (j % 3) as f64 * 0.3));
        cr.move_to(cx + r1 * a1.cos(), cy + r1 * a1.sin());
        cr.line_to(cx + r2 * a2.cos(), cy + r2 * a2.sin());
        if j % 2 == 0 {
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.30 * e);
            cr.set_line_width(1.4);
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.45 * e);
            cr.set_line_width(2.0);
        }
        let _ = cr.stroke();
    }
    cr.reset_clip();

    outline(cr, 0.0, 0.0);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.18 * e);
    cr.set_line_width(1.2);
    let _ = cr.stroke();
    cr.restore().ok();
}

fn draw(cr: &cairo::Context, fx: crate::win::Fx, colour: (f64, f64, f64), title: &str, body: &str, due: bool) {
    let (x, y) = (PAD, PAD);
    let (x1, y1) = (x + SIZE, y + SIZE);
    // peeling: the corner nearest where the note was grabbed curls up the
    // longer it is held and dragged; the bottom one always has a small curl
    let cb = CURL * (1.0 + if fx.peel_top { 0.0 } else { 1.8 * fx.peel });
    let ct = if fx.peel_top { CURL * 2.6 * fx.peel } else { 0.0 };

    cr.set_operator(cairo::Operator::Clear);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);

    if fx.crumple > 0.0 {
        crumpled(cr, fx.crumple, colour, title, body);
        return;
    }

    // soft drop shadow: stacked translucent copies of the outline
    for i in 0..9 {
        let grow = i as f64;
        cr.save().ok();
        cr.translate(2.0 + grow * 0.2 + fx.peel * 5.0, 5.0 + grow * 0.4 + (fx.lift - fx.press * 0.6) * 3.0 + fx.peel * 7.0);
        paper_path(cr, x - grow * 0.5, y - grow * 0.5, cb, ct);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.035);
        let _ = cr.fill();
        cr.restore().ok();
    }
    // the note lifts off the wall on hover and settles back when pressed
    cr.translate(0.0, fx.dy(4.0));

    // paper, lit from the top-left
    paper_path(cr, x, y, cb, ct);
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

    text(cr, x, y, title, body, 1.0);
    cr.reset_clip();

    flap(cr, x, y, cb, colour);
    if ct > 1.0 {
        // the top corner is the bottom one flipped over the note's horizontal axis
        cr.save().ok();
        cr.translate(0.0, 2.0 * y + SIZE);
        cr.scale(1.0, -1.0);
        flap(cr, x, y, ct, colour);
        cr.restore().ok();
    }

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
    // deleting crumples the note, drops it off the bottom of the screen, then trashes the file
    let hook = win::Hook::default();
    let (p, h) = (path.to_path_buf(), hook.clone());
    win::item(&menu, "Move to Trash", move || {
        let p = p.clone();
        match h.borrow().clone() {
            Some(go) => go(Box::new(move || win::trash(&p))),
            None => win::trash(&p),
        }
    });
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
        hook: Some(hook),
        menu,
    })
}
