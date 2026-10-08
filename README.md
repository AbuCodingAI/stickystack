# StickyStack

Your Desktop, as a fridge door.

- **Files become magnets.** Every file and folder in `~/Desktop` is shown as a small, glossy black magnet
  holding its icon (a `.desktop` launcher shows its own icon and starts the app). Hover to lift it, drag to
  move it, double-click to open it, right-click for more.
- **Sticky notes.** Click the round **+** to write a note. Notes are curled sticky notes you can edit,
  duplicate, copy, or send to the Trash from the right-click menu.
- **Reminders.** Start a line with a date, such as `10/5/31 I have to go to this event` (month/day/year,
  optional time like `3:30pm`; 9:00 if omitted), and a desktop notification appears when it comes due.

Notes are plain `.txt` files in `~/.local/share/stickystack/notes/`. Positions are remembered in
`~/.config/stickystack/`.

## Install

Arch Linux (AUR): `yay -S stickystack`

From source (needs Rust, GTK 3 and a notification daemon):

    cargo build --release
    install -Dm755 target/release/stickystack ~/.local/bin/stickystack

To start at login, copy `stickystack.desktop` to `~/.config/autostart/`.

## Notes

- X11 only for now (tested on XFCE). Wayland is not supported yet.
- On XFCE, hide the desktop's own icons so only magnets show:
  `xfconf-query -c xfce4-desktop -p /desktop-icons/style -n -t int -s 0`
- Run with `STICKYSTACK_DEBUG=1` to log every click the windows receive.

See [improvements.md](improvements.md) for the roadmap.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
