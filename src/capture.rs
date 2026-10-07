//! Screenshots of the desktop with the drawing on it, for copying and saving.
//!
//! The overlay is an ordinary layer surface, so a screenshot of the outputs already
//! includes the drawing. `grim` (wlroots compositors: Sway, Hyprland, Wayfire, ...) is
//! tried first because it is quick and never asks; when it is missing or fails, the
//! xdg-desktop-portal Screenshot interface (KDE and other portals) is used instead.
//!
//! A region is picked with `slurp` and taken with `grim -g`; the portal cannot take a
//! region without asking, so region capture needs both tools.

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

/// A screenshot as PNG data, or why there is none.
pub type Screenshot = Result<glib::Bytes, String>;

/// Where a screenshot goes once it is taken, called at most once.
type Done = Rc<RefCell<Option<Box<dyn FnOnce(Screenshot)>>>>;

/// Takes a screenshot of every output and hands the PNG to `done`.
pub fn screenshot(done: impl FnOnce(Screenshot) + 'static) {
    grim(&[], move |png| match png {
        Ok(png) => done(Ok(png)),
        Err(err) => {
            eprintln!("chicolli: grim failed ({err}), trying the screenshot portal");
            portal_screenshot(done);
        }
    });
}

/// Lets the user drag out a region with `slurp` and hands its geometry (`"x,y wxh"` in
/// layout coordinates, as `grim -g` takes it) to `done`, or `None` when they cancelled.
pub fn select_region(done: impl FnOnce(Result<Option<String>, String>) + 'static) {
    let slurp = gio::Subprocess::newv(
        &["slurp".as_ref()],
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    );
    let slurp = match slurp {
        Ok(slurp) => slurp,
        Err(err) => return done(Err(format!("capturing a region needs slurp ({err})"))),
    };
    slurp.clone().communicate_utf8_async(
        None,
        gio::Cancellable::NONE,
        move |result| match result {
            Ok((stdout, stderr)) => done(slurp_outcome(
                slurp.is_successful(),
                stdout.as_deref().unwrap_or_default(),
                stderr.as_deref().unwrap_or_default(),
            )),
            Err(err) => done(Err(format!("slurp failed ({err})"))),
        },
    );
}

/// What slurp's exit means: the picked geometry, `None` when the selection was
/// cancelled (Escape or a right click), or why slurp failed. slurp exits with 1 both
/// when cancelled and on errors, so its message tells them apart.
pub fn slurp_outcome(success: bool, stdout: &str, stderr: &str) -> Result<Option<String>, String> {
    let geometry = stdout.trim();
    if success {
        Ok((!geometry.is_empty()).then(|| geometry.to_owned()))
    } else if stderr.contains("selection cancelled") {
        Ok(None)
    } else {
        Err(format!("slurp failed ({})", stderr.trim()))
    }
}

/// Takes a screenshot of `geometry` (as `slurp` prints it) with `grim -g` and hands the
/// PNG to `done`.
pub fn screenshot_region(geometry: &str, done: impl FnOnce(Screenshot) + 'static) {
    grim(&["-g", geometry], move |png| {
        done(png.map_err(|err| format!("capturing a region needs grim ({err})")))
    });
}

/// Runs `grim <args> -` and hands its PNG to `done`, or grim's error message.
fn grim(args: &[&str], done: impl FnOnce(Screenshot) + 'static) {
    let argv: Vec<&std::ffi::OsStr> = std::iter::once("grim")
        .chain(args.iter().copied())
        .chain(std::iter::once("-"))
        .map(std::ffi::OsStr::new)
        .collect();
    let grim = match gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    ) {
        Ok(grim) => grim,
        Err(err) => return done(Err(err.to_string())),
    };
    grim.clone()
        .communicate_async(None, gio::Cancellable::NONE, move |result| match result {
            Ok((Some(png), _)) if grim.is_successful() && !png.is_empty() => done(Ok(png)),
            // E.g. a compositor without wlr-screencopy.
            Ok((_, stderr)) => done(Err(stderr
                .map(|err| String::from_utf8_lossy(&err).trim().to_owned())
                .unwrap_or_default())),
            Err(err) => done(Err(err.to_string())),
        });
}

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
/// How long to wait for the portal's answer before giving up, so a portal that never
/// answers (or a permission dialog nobody can reach under the overlay) does not block
/// copy and save for good.
const PORTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Asks xdg-desktop-portal for a non-interactive screenshot and reads the file it writes.
fn portal_screenshot(done: impl FnOnce(Screenshot) + 'static) {
    gio::bus_get(
        gio::BusType::Session,
        gio::Cancellable::NONE,
        move |bus| match bus {
            Ok(bus) => portal_request(&bus, done),
            Err(err) => done(Err(format!(
                "grim is missing or failed, and the session bus is not reachable ({err})"
            ))),
        },
    );
}

fn portal_request(bus: &gio::DBusConnection, done: impl FnOnce(Screenshot) + 'static) {
    // The reply arrives as a Response signal on a request object whose path follows from
    // our bus name and a token we pick, so subscribe before calling to never miss it.
    let token = format!("chicolli_{}", glib::random_int());
    let sender = bus
        .unique_name()
        .map(|name| name.trim_start_matches(':').replace('.', "_"))
        .unwrap_or_default();
    let request_path = format!("{PORTAL_PATH}/request/{sender}/{token}");

    let done: Done = Rc::new(RefCell::new(Some(Box::new(done))));
    let finish = {
        let done = done.clone();
        move |result: Screenshot| {
            if let Some(done) = done.borrow_mut().take() {
                done(result);
            }
        }
    };
    let subscription = Rc::new(RefCell::new(None));
    *subscription.borrow_mut() = Some(bus.subscribe_to_signal(
        Some(PORTAL),
        Some("org.freedesktop.portal.Request"),
        Some("Response"),
        Some(&request_path),
        None,
        gio::DBusSignalFlags::NONE,
        glib::clone!(
            #[strong]
            subscription,
            #[strong]
            finish,
            move |signal| {
                // One response per request; unsubscribing also frees this closure.
                subscription.borrow_mut().take();
                let finish = finish.clone();
                let response = signal
                    .parameters
                    .get::<(u32, HashMap<String, glib::Variant>)>();
                let uri = match response {
                    Some((0, results)) => results.get("uri").and_then(|uri| uri.get::<String>()),
                    Some((1, _)) => return finish(Err("the screenshot was cancelled".into())),
                    _ => None,
                };
                let Some(uri) = uri else {
                    return finish(Err("the screenshot portal failed".into()));
                };
                gio::File::for_uri(&uri).load_bytes_async(gio::Cancellable::NONE, move |result| {
                    finish(
                        result
                            .map(|(png, _)| png)
                            .map_err(|err| format!("reading {uri}: {err}")),
                    )
                });
            },
        ),
    ));

    glib::timeout_add_local_once(
        PORTAL_TIMEOUT,
        glib::clone!(
            #[strong]
            subscription,
            #[strong]
            finish,
            move || {
                // A no-op if the portal already answered.
                subscription.borrow_mut().take();
                finish(Err("the screenshot portal did not answer".into()));
            }
        ),
    );

    let options = HashMap::from([
        ("handle_token", token.to_variant()),
        ("interactive", false.to_variant()),
    ]);
    bus.call(
        Some(PORTAL),
        PORTAL_PATH,
        "org.freedesktop.portal.Screenshot",
        "Screenshot",
        Some(&("", options).to_variant()),
        Some(glib::VariantTy::new("(o)").unwrap()),
        gio::DBusCallFlags::NONE,
        -1,
        gio::Cancellable::NONE,
        move |reply| {
            if let Err(err) = reply {
                subscription.borrow_mut().take();
                finish(Err(format!(
                    "grim is missing or failed, and so did the screenshot portal ({err})"
                )));
            }
        },
    );
}

/// Puts the PNG on the clipboard and reports how that went. `wl-copy` keeps serving it
/// after chicolli exits, so it can be pasted once the overlay is closed; without it GTK's
/// clipboard is used, which only lasts while chicolli runs (unless a clipboard manager
/// takes it over).
pub fn copy_to_clipboard(
    png: glib::Bytes,
    display: gtk::gdk::Display,
    subject: &'static str,
    done: impl FnOnce(Result<String, String>) + 'static,
) {
    let gtk_clipboard = move |png: &glib::Bytes| match gtk::gdk::Texture::from_bytes(png) {
        Ok(texture) => {
            display.clipboard().set_texture(&texture);
            Ok(format!(
                "Copied {subject} to the clipboard until chicolli quits \
                 (install wl-clipboard to keep it after that)"
            ))
        }
        Err(err) => Err(format!("copying to the clipboard: {err}")),
    };
    let wl_copy = gio::Subprocess::newv(
        &["wl-copy".as_ref(), "--type".as_ref(), "image/png".as_ref()],
        gio::SubprocessFlags::STDIN_PIPE,
    );
    let Ok(wl_copy) = wl_copy else {
        return done(gtk_clipboard(&png));
    };
    wl_copy
        .clone()
        .communicate_async(Some(&png.clone()), gio::Cancellable::NONE, move |result| {
            if result.is_ok() && wl_copy.is_successful() {
                done(Ok(format!("Copied {subject} to the clipboard")));
            } else {
                eprintln!("chicolli: wl-copy failed, using GTK's clipboard instead");
                done(gtk_clipboard(&png));
            }
        });
}

/// Where Ctrl+S saves: the pictures folder (XDG `PICTURES`, else `~/Pictures`), as
/// `chicolli-<stamp>.png`, with `-2`, `-3`, ... added if that name is taken.
pub fn save_path(dir: &Path, stamp: &str) -> PathBuf {
    let mut path = dir.join(format!("chicolli-{stamp}.png"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("chicolli-{stamp}-{n}.png"));
        n += 1;
    }
    path
}

/// Saves the PNG in the pictures folder and returns its path.
pub fn save(png: &glib::Bytes) -> Result<PathBuf, String> {
    let dir = dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
        .ok_or("no home folder")?;
    std::fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let stamp = glib::DateTime::now_local()
        .and_then(|now| now.format("%Y-%m-%d_%H-%M-%S"))
        .map_err(|err| err.to_string())?;
    let path = save_path(&dir, &stamp);
    std::fs::write(&path, png).map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(path)
}
