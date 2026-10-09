//! The "+" button: a small glossy round button that starts a new note.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;


use crate::store::Positions;

const SIZE: f64 = 56.0;
const PAD: f64 = 12.0;

fn draw(cr: &cairo::Context, fx: crate::win::Fx) {
    let (c, r) = (PAD + SIZE / 2.0, SIZE / 2.0);
    cr.set_operator(cairo::Operator::Clear);
    let _ = cr.paint();
    cr.set_operator(cairo::Operator::Over);

    for i in 0..7 {
        cr.arc(c + 1.0, c + 4.0 + fx.lift * 3.0, r + i as f64 * 0.6, 0.0, 2.0 * PI);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.04);
        let _ = cr.fill();
    cr.translate(0.0, fx.dy(3.0));
    }
    cr.arc(c, c + 3.0, r, 0.0, 2.0 * PI);
    cr.set_source_rgb(0.62, 0.45, 0.02);
    let _ = cr.fill();

    cr.arc(c, c, r, 0.0, 2.0 * PI);
    let face = cairo::RadialGradient::new(c - r * 0.35, c - r * 0.4, 2.0, c, c, r * 1.1);
    face.add_color_stop_rgb(0.0, 1.0, 0.92, 0.45);
    face.add_color_stop_rgb(1.0, 0.92, 0.66, 0.08);
    let _ = cr.set_source(&face);
    let _ = cr.fill_preserve();
    cr.clip();

    cr.set_source_rgb(0.35, 0.24, 0.02);
    cr.set_line_width(5.0);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.move_to(c - 12.0, c);
    cr.line_to(c + 12.0, c);
    cr.move_to(c, c - 12.0);
    cr.line_to(c, c + 12.0);
    let _ = cr.stroke();

    cr.new_path();
    cr.move_to(c - r, c - r);
    cr.line_to(c + r, c - r);
    cr.line_to(c + r, c - r * 0.1);
    cr.curve_to(c + r * 0.5, c + r * 0.15, c - r * 0.5, c + r * 0.15, c - r, c - r * 0.1);
    cr.close_path();
    let gloss = cairo::LinearGradient::new(c, c - r, c, c);
    gloss.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.55);
    gloss.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.05);
    let _ = cr.set_source(&gloss);
    let _ = cr.fill();
}

pub fn create(positions: Rc<RefCell<Positions>>) -> gtk::Window {
    let (w, _) = crate::win::screen_size();
    crate::win::build(crate::win::Spec {
        key: "+new",
        title: "New note",
        tooltip: Some("New note".into()),
        size: (SIZE + PAD * 2.0) as i32,
        slot: 0,
        origin: Some((w - 110, 40)),
        single_click: true,
        positions,
        draw: Box::new(draw),
        on_open: Box::new(|| crate::editor::open(None)),
        hook: None,
        menu: gtk::Menu::new(),
    })
}
