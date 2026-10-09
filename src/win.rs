//! Shared plumbing for every floating thing on the desktop: a borderless,
//! translucent window that sits below other windows, reacts to the pointer
//! (lifts on hover, presses down on click), can be dragged anywhere and
//! remembers where it was left.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;

use crate::store::Positions;

/// Pointer-driven animation state handed to every draw function.
#[derive(Clone, Copy, Default)]
pub struct Fx {
    /// 0..1, eases toward 1 while the pointer is over the window.
    pub lift: f64,
    /// 0..1, spikes to 1 on click and eases back.
    pub press: f64,
    /// 0..1, rises while the window is being dragged and eases back on release.
    pub peel: f64,
    /// Peel from the top corner (grabbed in the top half) instead of the bottom.
    pub peel_top: bool,
    /// 0..1, how far the note has balled up while being deleted.
    pub crumple: f64,
}

impl Fx {
    /// Vertical offset for the object itself (negative = raised).
    pub fn dy(&self, rise: f64) -> f64 {
        -self.lift * rise + self.press * rise * 0.8
    }
}

#[derive(Clone, Copy)]
struct Drag {
    ox: f64,
    oy: f64,
    sx: f64,
    sy: f64,
    active: bool,
}

const CRUMPLE_SECS: f64 = 0.45; // time to ball the note up
const GRAVITY: f64 = 2600.0; // px/s² once it lets go

pub type Hook = Rc<RefCell<Option<Rc<dyn Fn(Box<dyn FnOnce()>)>>>>;

pub struct Spec<'a> {
    /// Unique id, used to remember the window's position.
    pub key: &'a str,
    pub title: &'a str,
    pub tooltip: Option<String>,
    pub size: i32,
    /// Index used to pick a default grid cell when there's no saved position.
    pub slot: usize,
    /// Fixed default position, overriding the grid.
    pub origin: Option<(i32, i32)>,
    /// A press triggers `on_open` immediately and never drags (for buttons).
    pub single_click: bool,
    pub positions: Rc<RefCell<Positions>>,
    pub draw: Box<dyn Fn(&cairo::Context, Fx)>,
    pub on_open: Box<dyn Fn()>,
    pub menu: gtk::Menu,
    /// Filled in by `build` with a function that plays the delete animation
    /// (crumple, fall off the bottom of the screen) and then runs the callback.
    pub hook: Option<Hook>,
}

pub fn screen_size() -> (i32, i32) {
    gdk::Display::default()
        .and_then(|d| d.primary_monitor().or_else(|| d.monitor(0)))
        .map(|m| (m.geometry().width(), m.geometry().height()))
        .unwrap_or((1920, 1080))
}

pub fn trash(path: &Path) {
    let _ = gtk::gio::File::for_path(path).trash(gtk::gio::Cancellable::NONE);
}

pub fn copy_text(text: &str) {
    gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD).set_text(text);
}

/// A menu item wired to a callback.
pub fn item(menu: &gtk::Menu, label: &str, f: impl Fn() + 'static) {
    let it = gtk::MenuItem::with_label(label);
    it.connect_activate(move |_| f());
    menu.append(&it);
}

pub fn separator(menu: &gtk::Menu) {
    menu.append(&gtk::SeparatorMenuItem::new());
}

fn set_cursor(win: &gtk::Window, name: &str) {
    if let (Some(gw), Some(display)) = (win.window(), gdk::Display::default()) {
        gw.set_cursor(gdk::Cursor::from_name(&display, name).as_ref());
    }
}

pub fn build(spec: Spec) -> gtk::Window {
    let Spec { key, title, tooltip, size, slot, origin, single_click, positions, draw, on_open, menu, hook } = spec;
    let debug = std::env::var_os("STICKYSTACK_DEBUG").is_some();

    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title(title);
    win.set_default_size(size, size);
    win.set_decorated(false);
    win.set_resizable(false);
    win.set_app_paintable(true);
    win.set_skip_taskbar_hint(true);
    win.set_skip_pager_hint(true);
    win.set_keep_below(true);
    win.stick();
    win.set_type_hint(gdk::WindowTypeHint::Utility);
    win.set_accept_focus(false);
    if let Some(visual) = WidgetExt::screen(&win).and_then(|s| s.rgba_visual()) {
        win.set_visual(Some(&visual));
    }
    if let Some(t) = tooltip {
        win.set_tooltip_text(Some(&t));
    }

    // default spot: columns down the left side, unless the user moved it before
    let (_, sh) = screen_size();
    let step = size + 4;
    let rows = ((sh - 80) / step).max(1) as usize;
    let grid = (40 + (slot / rows) as i32 * step, 40 + (slot % rows) as i32 * step);
    let (px, py) = positions.borrow().get(key).or(origin).unwrap_or(grid);
    win.move_(px, py);

    // animation state: current value and target for hover and press
    let fx = Rc::new(Cell::new(Fx::default()));
    let target = Rc::new(Cell::new((0.0f64, 0.0f64, 0.0f64))); // lift, press, peel
    let running = Rc::new(Cell::new(false));

    let area = gtk::DrawingArea::new();
    area.set_size_request(size, size);
    let fx_draw = fx.clone();
    area.connect_draw(move |_, cr| {
        draw(cr, fx_draw.get());
        glib::Propagation::Stop
    });
    win.add(&area);

    // runs ~60fps only while something is still moving
    let kick: Rc<dyn Fn()> = {
        let (fx, target, running, area) = (fx.clone(), target.clone(), running.clone(), area.clone());
        Rc::new(move || {
            if running.replace(true) {
                return;
            }
            let (fx, target, running, area) = (fx.clone(), target.clone(), running.clone(), area.clone());
            glib::timeout_add_local(Duration::from_millis(16), move || {
                let (tl, tp, tpeel) = target.get();
                let mut f = fx.get();
                f.lift += (tl - f.lift) * 0.35;
                f.press += (tp - f.press) * 0.45;
                f.peel += (tpeel - f.peel) * if tpeel > f.peel { 0.14 } else { 0.30 };
                let settled = (tl - f.lift).abs() < 0.01 && (tp - f.press).abs() < 0.01 && (tpeel - f.peel).abs() < 0.01;
                if settled {
                    f.lift = tl;
                    f.press = tp;
                    f.peel = tpeel;
                }
                fx.set(f);
                area.queue_draw();
                if settled {
                    running.set(false);
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        })
    };

    win.add_events(
        gdk::EventMask::BUTTON_PRESS_MASK
            | gdk::EventMask::BUTTON_RELEASE_MASK
            | gdk::EventMask::BUTTON_MOTION_MASK
            | gdk::EventMask::ENTER_NOTIFY_MASK
            | gdk::EventMask::LEAVE_NOTIFY_MASK,
    );

    let (t, k) = (target.clone(), kick.clone());
    win.connect_enter_notify_event(move |w, _| {
        set_cursor(w, if single_click { "pointer" } else { "grab" });
        t.set((1.0, t.get().1, t.get().2));
        k();
        glib::Propagation::Proceed
    });
    let (t, k) = (target.clone(), kick.clone());
    win.connect_leave_notify_event(move |_, _| {
        t.set((0.0, 0.0, t.get().2));
        k();
        glib::Propagation::Proceed
    });

    // Dragging is done by hand (not by asking the window manager to move us),
    // so compositor effects like wobbly windows never kick in. The press point
    // is remembered as (pointer offset inside the window, pointer start).
    let dying = Rc::new(Cell::new(false));
    let drag: Rc<Cell<Option<Drag>>> = Rc::new(Cell::new(None));

    let (t, k, d) = (target.clone(), kick.clone(), drag.clone());
    let (fx_press, dying_press) = (fx.clone(), dying.clone());
    win.connect_button_press_event(move |w, e| {
        if dying_press.get() {
            return glib::Propagation::Stop;
        }
        if debug {
            eprintln!("stickystack: press button={} type={:?}", e.button(), e.event_type());
        }
        let squish = |t: &Rc<Cell<(f64, f64, f64)>>, k: &Rc<dyn Fn()>| {
            t.set((t.get().0, 1.0, t.get().2));
            k();
            let (t, k) = (t.clone(), k.clone());
            glib::timeout_add_local_once(Duration::from_millis(130), move || {
                t.set((t.get().0, 0.0, t.get().2));
                k();
            });
        };
        match (e.button(), e.event_type()) {
            (1, gdk::EventType::ButtonPress) if single_click => {
                squish(&t, &k);
                on_open();
            }
            (1, gdk::EventType::DoubleButtonPress) if !single_click => {
                squish(&t, &k);
                on_open();
            }
            (1, gdk::EventType::ButtonPress) => {
                squish(&t, &k);
                let (rx, ry) = e.root();
                let (wx, wy) = w.position();
                let top = e.position().1 < size as f64 / 2.0;
                fx_press.set(Fx { peel_top: top, ..fx_press.get() });
                d.set(Some(Drag { ox: rx - wx as f64, oy: ry - wy as f64, sx: rx, sy: ry, active: false }));
            }
            (3, gdk::EventType::ButtonPress) if !single_click => {
                squish(&t, &k);
                menu.popup_at_pointer(Some(e));
            }
            _ => {}
        }
        glib::Propagation::Stop
    });

    // follow the pointer; past a few pixels it counts as a drag and the note peels
    let (t, k, d) = (target.clone(), kick.clone(), drag.clone());
    win.connect_motion_notify_event(move |w, e| {
        if let Some(mut s) = d.get() {
            let (rx, ry) = e.root();
            if !s.active && (rx - s.sx).hypot(ry - s.sy) > 4.0 {
                s.active = true;
                set_cursor(w, "grabbing");
                t.set((t.get().0, t.get().1, 1.0));
                k();
            }
            if s.active {
                w.move_((rx - s.ox).round() as i32, (ry - s.oy).round() as i32);
            }
            d.set(Some(s));
        }
        glib::Propagation::Proceed
    });

    // let go: press the note back down onto the wall
    let (t, k, d) = (target.clone(), kick.clone(), drag.clone());
    win.connect_button_release_event(move |w, e| {
        if e.button() == 1 && d.take().is_some() {
            t.set((t.get().0, t.get().1, 0.0));
            k();
            set_cursor(w, "grab");
        }
        glib::Propagation::Proceed
    });

    // remember where the window was left, once it stops moving
    let key = key.to_string();
    let pending = Rc::new(Cell::new(false));
    let dying_cfg = dying.clone();
    win.connect_configure_event(move |w, _| {
        if dying_cfg.get() {
            return false;
        }
        if !pending.replace(true) {
            let (w, key, positions, pending) = (w.clone(), key.clone(), positions.clone(), pending.clone());
            glib::timeout_add_local_once(Duration::from_millis(400), move || {
                pending.set(false);
                positions.borrow_mut().set(&key, w.position());
            });
        }
        false
    });

    // delete animation: ball the note up, then let it fall off the bottom of the screen
    if let Some(hook) = hook {
        let (weak, fx, area, dying) = (win.downgrade(), fx.clone(), area.clone(), dying.clone());
        *hook.borrow_mut() = Some(Rc::new(move |done: Box<dyn FnOnce()>| {
            let Some(w) = weak.upgrade() else { return done() };
            if dying.replace(true) {
                return;
            }
            let (x0, y0) = w.position();
            let (_, screen_h) = screen_size();
            let (start, done, fx, area) = (Instant::now(), Cell::new(Some(done)), fx.clone(), area.clone());
            glib::timeout_add_local(Duration::from_millis(16), move || {
                let el = start.elapsed().as_secs_f64();
                let mut f = fx.get();
                f.crumple = (el / CRUMPLE_SECS).clamp(0.001, 1.0);
                fx.set(f);
                area.queue_draw();
                if el > CRUMPLE_SECS {
                    let t = el - CRUMPLE_SECS;
                    let y = y0 as f64 + 0.5 * GRAVITY * t * t;
                    w.move_(x0, y as i32);
                    if y > screen_h as f64 {
                        w.hide();
                        if let Some(d) = done.take() {
                            d();
                        }
                        return glib::ControlFlow::Break;
                    }
                }
                glib::ControlFlow::Continue
            });
        }));
    }

    win.show_all();
    set_cursor(&win, if single_click { "pointer" } else { "grab" });
    win.move_(px, py);
    win
}
