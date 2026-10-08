# StickyStack: 25 ways to improve it

Grouped by area. Items marked **(verify)** are things already built but not yet tested against a real pointer.

## Reliability and testing

1. **Verify the interactions (verify).** Hover lift, press-down, right-click menus, double-click launch and the note editor have only been checked visually. Test each against a real pointer and fix whatever fails.
2. **Automated render tests.** Render each window to a PNG in a headless run and compare against saved reference images, so visual regressions get caught.
3. **Unit tests for reminders.** Cover date parsing (`M/D/YY`, `M/D/YYYY`, `3:30pm`, invalid dates like 2/30) and the "already reminded" log.
4. **Single-instance guard.** Starting a second copy currently draws every window twice. Use a lock file or D-Bus name so a second launch just exits.
5. **Crash-safe saving.** Write notes and positions to a temporary file and rename it into place, so a crash mid-write can't leave an empty note.

## Desktop behaviour

6. **Correct window layer.** Use the real desktop window type so magnets sit above the wallpaper but below everything, including when "Show Desktop" is used. Right now VS Code simply covers them.
7. **Multi-monitor support.** Remember which monitor each item is on, and handle monitors being plugged or unplugged without items landing off-screen.
8. **Rescale on resolution change.** Clamp saved positions to the current screen, so items never get stranded after a resolution switch.
9. **Snap to grid and tidy up.** A right-click desktop action to line everything up, plus optional snapping while dragging.
10. **Wayland support.** Everything here relies on X11 window positioning. Add a layer-shell path so it works on Wayland desktops.

## Magnets

11. **Per-magnet customisation.** Right-click to change magnet colour or shape, saved per file.
12. **Folder previews.** Show a tiny stack of the folder's contents, or its item count, on the magnet.
13. **Image thumbnails.** Show the actual picture on the magnet for image files, not the generic icon.
14. **Drag files onto magnets.** Dropping a file onto a folder magnet moves it in. Dropping onto an app launcher opens the file with that app.
15. **Running indicator.** A small glowing dot when a launcher's app is already open, and clicking it focuses the existing window.

## Notes and reminders

16. **Reminder snooze.** Notification actions for "Snooze 10 min" and "Done", instead of firing once and forgetting.
17. **Recurring reminders.** Syntax such as `every mon 9:00` or `daily 8:00`.
18. **Natural dates.** Accept `tomorrow 3pm` and `next friday` alongside numeric dates.
19. **Note colours and sizes.** Choose a colour from the menu, and let long notes grow instead of truncating after six lines.
20. **Checklists in notes.** Lines starting with `[ ]` become tickable checkboxes right on the note.
21. **Trash undo.** A short "Note moved to Trash. Undo" toast after deleting.

## Polish and packaging

22. **Settings window.** One place for magnet size, note size, notification sound, reminder default hour and autostart. Today these are constants in the code.
23. **System tray icon.** Show or hide everything, create a note and quit, without having to find the process.
24. **Smoother animation.** Slight tilt toward the cursor on hover and a gentle settle bounce after dropping, drawn with easing curves, not simple smoothing.
25. **Proper packaging.** An Arch `PKGBUILD`, an app icon and a one-command install script, so setup isn't a manual copy to `~/.local/bin`.
