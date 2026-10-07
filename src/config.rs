use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Error, Read},
};

use dirs::config_dir;

/// The settings in effect: what the config file sets, with defaults for the rest.
#[derive(Debug, Clone, PartialEq)]
pub struct Configuration {
    /// Stroke width in pixels.
    pub line_width: f64,
    pub keys: Keys,
}

/// Key for each action, as a GTK key name with optional modifiers in front, such as
/// `"1"`, `"F1"` or `"<Ctrl>z"`. An empty string leaves the action unbound.
#[derive(Debug, Clone, PartialEq)]
pub struct Keys {
    pub pen: String,
    pub arrow: String,
    pub reverse_arrow: String,
    pub rectangle: String,
    pub text: String,
    pub highlighter: String,
    pub eraser: String,
    pub pass_through: String,
    pub red: String,
    pub green: String,
    pub blue: String,
    pub color_chooser: String,
    pub undo: String,
    pub redo: String,
    pub clear: String,
    /// Copy the screen with the drawing to the clipboard.
    pub copy: String,
    /// Save the screen with the drawing as a PNG in the pictures folder.
    pub save: String,
    /// Close chicolli. Escape only quits when no text label is being typed.
    pub quit: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            line_width: 5.0,
            keys: Keys {
                pen: "1".into(),
                arrow: "2".into(),
                reverse_arrow: "3".into(),
                rectangle: "4".into(),
                text: "5".into(),
                highlighter: "6".into(),
                eraser: "e".into(),
                pass_through: "d".into(),
                red: "r".into(),
                green: "g".into(),
                blue: "b".into(),
                color_chooser: "c".into(),
                undo: "<Ctrl>z".into(),
                redo: "<Ctrl><Shift>z".into(),
                clear: "<Ctrl>x".into(),
                copy: "<Ctrl>c".into(),
                save: "<Ctrl>s".into(),
                quit: "Escape".into(),
            },
        }
    }
}

/// Line widths the config file may set; the toolbar and scroll wheel keep to the same range.
pub const MIN_LINE_WIDTH: f64 = 1.0;
pub const MAX_LINE_WIDTH: f64 = 200.0;

/// The file as written; every option may be missing.
#[derive(Deserialize, Default)]
#[serde(default)]
struct RawConfig {
    line_width: Option<f64>,
    keys: RawKeys,
    // The flat names used before the "keys" group, still read so older files keep working.
    line_thickness: Option<f64>,
    draw_keybind: Option<String>,
    arrow_keybind: Option<String>,
    reverse_arrow_keybind: Option<String>,
    rectangle_keybind: Option<String>,
    text_keybind: Option<String>,
    highlighter_keybind: Option<String>,
    disable_drawing: Option<String>,
    color_r: Option<String>,
    color_g: Option<String>,
    color_b: Option<String>,
    color_chooser: Option<String>,
    undo: Option<String>,
    clear_all: Option<String>,
    #[serde(flatten)]
    unknown: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawKeys {
    pen: Option<String>,
    arrow: Option<String>,
    reverse_arrow: Option<String>,
    rectangle: Option<String>,
    text: Option<String>,
    highlighter: Option<String>,
    eraser: Option<String>,
    pass_through: Option<String>,
    red: Option<String>,
    green: Option<String>,
    blue: Option<String>,
    color_chooser: Option<String>,
    undo: Option<String>,
    redo: Option<String>,
    clear: Option<String>,
    copy: Option<String>,
    save: Option<String>,
    quit: Option<String>,
    #[serde(flatten)]
    unknown: BTreeMap<String, serde_json::Value>,
}

/// The old flat `undo` and `clear_all` options were always pressed with Ctrl.
fn with_ctrl(key: String) -> String {
    if key.is_empty() || key.starts_with('<') {
        key
    } else {
        format!("<Ctrl>{key}")
    }
}

/// Parses a config file's contents. Options it leaves out get their defaults; the
/// returned list names options that were ignored because chicolli does not know them.
pub fn parse_config(content: &str) -> Result<(Configuration, Vec<String>), serde_json::Error> {
    let raw: RawConfig = serde_json::from_str(content)?;
    let k = raw.keys;
    let d = Configuration::default();
    let pick =
        |new: Option<String>, old: Option<String>, default: String| new.or(old).unwrap_or(default);
    let config = Configuration {
        line_width: raw
            .line_width
            .or(raw.line_thickness)
            .filter(|&width| {
                let valid = (MIN_LINE_WIDTH..=MAX_LINE_WIDTH).contains(&width);
                if !valid {
                    eprintln!(
                        "chicolli: line_width must be between {MIN_LINE_WIDTH} and {MAX_LINE_WIDTH}, using {}",
                        d.line_width
                    );
                }
                valid
            })
            .unwrap_or(d.line_width),
        keys: Keys {
            pen: pick(k.pen, raw.draw_keybind, d.keys.pen),
            arrow: pick(k.arrow, raw.arrow_keybind, d.keys.arrow),
            reverse_arrow: pick(
                k.reverse_arrow,
                raw.reverse_arrow_keybind,
                d.keys.reverse_arrow,
            ),
            rectangle: pick(k.rectangle, raw.rectangle_keybind, d.keys.rectangle),
            text: pick(k.text, raw.text_keybind, d.keys.text),
            highlighter: pick(k.highlighter, raw.highlighter_keybind, d.keys.highlighter),
            eraser: k.eraser.unwrap_or(d.keys.eraser),
            pass_through: pick(k.pass_through, raw.disable_drawing, d.keys.pass_through),
            red: pick(k.red, raw.color_r, d.keys.red),
            green: pick(k.green, raw.color_g, d.keys.green),
            blue: pick(k.blue, raw.color_b, d.keys.blue),
            color_chooser: pick(k.color_chooser, raw.color_chooser, d.keys.color_chooser),
            undo: pick(k.undo, raw.undo.map(with_ctrl), d.keys.undo),
            redo: k.redo.unwrap_or(d.keys.redo),
            clear: pick(k.clear, raw.clear_all.map(with_ctrl), d.keys.clear),
            copy: k.copy.unwrap_or(d.keys.copy),
            save: k.save.unwrap_or(d.keys.save),
            quit: k.quit.unwrap_or(d.keys.quit),
        },
    };
    let unknown = raw
        .unknown
        .into_keys()
        .chain(k.unknown.into_keys().map(|name| format!("keys.{name}")))
        .collect();
    Ok((config, unknown))
}

const CONFIG_NAME: &str = "chicolli.json";
const CONFIG_DIR: &str = "chicolli";

/// A new config file starts out empty, so every option follows the defaults, including
/// defaults changed by later versions; the README lists what can go in it.
const NEW_CONFIG: &str = "{\n  \"keys\": {}\n}\n";

fn write_default_config(path: &std::path::Path) -> Result<(), Error> {
    std::fs::write(path, NEW_CONFIG)
}

pub fn get_config() -> Configuration {
    match read_config() {
        Ok(conf) => conf,
        Err(r) => {
            eprintln!(
                "chicolli: could not read or create the config file, using the defaults: {}",
                r
            );
            Configuration::default()
        }
    }
}

/// Path of the config file, `~/.config/chicolli/chicolli.json`; it may not exist yet.
pub fn config_file_path() -> Option<std::path::PathBuf> {
    let mut path = config_dir()?;
    path.push(CONFIG_DIR);
    path.push(CONFIG_NAME);
    Some(path)
}

pub fn read_config() -> Result<Configuration, Error> {
    // get the config dir path
    let conf_path = config_dir();
    match conf_path {
        Some(mut conf_path) => {
            // append the dir name and check if exists
            conf_path.push(CONFIG_DIR);
            if conf_path.as_path().exists() {
                // append the name and check if exists
                conf_path.push(CONFIG_NAME);
                if conf_path.as_path().exists() {
                    // parse the config and return
                    let config = read_config_file(conf_path.as_path())?;
                    Ok(config)
                } else {
                    write_default_config(conf_path.as_path())?;
                    read_config()
                }
            } else {
                std::fs::create_dir_all(conf_path.as_path())?;
                conf_path.push(CONFIG_NAME);
                write_default_config(conf_path.as_path())?;
                read_config()
            }
        }
        None => Err(Error::other("could not find the config directory")),
    }
}

/// Reads and parses a config file, filling options it leaves out with the defaults.
pub fn read_config_file(file_path: &std::path::Path) -> Result<Configuration, Error> {
    let mut file = File::open(file_path)?;

    // Read the content of the file into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let (config, unknown) = parse_config(&content)?;
    for option in unknown {
        eprintln!(
            "chicolli: ignoring unknown option {option:?} in {}",
            file_path.display()
        );
    }

    Ok(config)
}
