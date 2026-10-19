# 0020. Global hotkey on Wayland via a desktop shortcut

- **Status:** Accepted
- **Date:** 2026-10-19

## Context

The global hotkey is registered with `tauri-plugin-global-shortcut`, which uses the
`global-hotkey` crate. On Linux that crate supports X11 only: it grabs the key with `XGrabKey`.
Under a Wayland session the grab goes to XWayland and *succeeds*, but XWayland sees keys only while
one of its own windows has focus, so the hotkey never fires. Registration reports no error, and
the settings would show a hotkey that does nothing. The default sessions of Ubuntu, Fedora, Debian
GNOME and KDE Plasma 6 are Wayland. Running the app on XWayland (`GDK_BACKEND=x11`) does not help,
for the same reason.

Wayland's way for an application to get a global shortcut is the XDG `GlobalShortcuts` portal.
GNOME 48 and later, KDE Plasma and Hyprland support it; wlroots compositors such as Sway, and Xfce,
Cinnamon and COSMIC, do not. The application only suggests a trigger and the portal asks the user
in its own dialog, so the hotkey recorder in the settings would not apply. For an application that
is not a Flatpak the portal also needs an application id that matches its `.desktop` file. It is a
new dependency, an asynchronous session, new settings UI and testing on each desktop.

Every Wayland desktop lets the user bind a key to a command (GNOME Settings → Keyboard → Custom
Shortcuts, KDE System Settings → Shortcuts, `bindsym`/`bind` in Sway and Hyprland). Starting
`quicklly` while it runs already reaches the running instance through
`tauri-plugin-single-instance`, which shows the input window.

## Decision

On a Wayland session Quicklly registers no global hotkey. The user binds the command `quicklly`
in the desktop's own keyboard settings, and the second launch it starts is the hotkey.

- **Wayland session** means that `WAYLAND_DISPLAY` is set (`settings::hotkey_in_desktop`, Linux
  only). A Wayland compositor sets it for every program it runs. `XDG_SESSION_TYPE` is not used:
  it comes from the login manager, and a compositor started from a console, such as Sway, often
  leaves it at `tty`. Nor is it the display backend GTK picked: on XWayland the grab fails just the
  same.
- `register_hotkey` registers nothing there, and a hotkey change is only saved.
- The settings page's **Hotkey** row says that the hotkey is set in the system settings and names
  the command to run; it records and resets nothing. The help says the same on Linux.
- A second launch on Wayland opens what **Hotkey opens** names, the input window or the Pad, as
  the hotkey does. Everywhere else, Windows included, a second launch still opens the input window.

X11 sessions keep the plugin's hotkey as on Windows.

## Consequences

- The hotkey works on every Wayland desktop that can run a command on a key, with no new
  dependency and no new code path for it.
- The key is chosen outside Quicklly, in a different place on every desktop, so the README has to
  explain it per desktop. The command must be on the `PATH` (deb and rpm put it there; an AppImage
  is bound by its full path).
- Each press starts a short-lived process that hands over to the running one, so the window
  comes up a little later than with a grabbed key.
- The second process may receive an activation token from the desktop (`XDG_ACTIVATION_TOKEN`),
  but single-instance forwards only the arguments and the working folder. If a desktop refuses to
  focus the window without it, the `GlobalShortcuts` portal, whose activation signal carries a
  token, is the next step, in a new decision.
- A hotkey saved in the settings stays in `settings.json` and is used again in an X11 session.
