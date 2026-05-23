use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Error, Read},
};

use dirs::config_dir;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub line_thickness: Option<f64>,
    pub draw_keybind: Option<String>,
    pub arrow_keybind: Option<String>,
    pub reverse_arrow_keybind: Option<String>,
    pub rectangle_keybind: Option<String>,
    pub text_keybind: Option<String>,
    pub highlighter_keybind: Option<String>,
    pub disable_drawing: Option<String>,
    pub color_r: Option<String>,
    pub color_g: Option<String>,
    pub color_b: Option<String>,
    pub color_chooser: Option<String>,
    pub undo: Option<String>,
    pub clear_all: Option<String>,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            line_thickness: Some(5.0),
            draw_keybind: Some(String::from("1")),
            arrow_keybind: Some(String::from("2")),
            reverse_arrow_keybind: Some(String::from("3")),
            rectangle_keybind: Some(String::from("4")),
            text_keybind: Some(String::from("5")),
            highlighter_keybind: Some(String::from("6")),
            disable_drawing: Some(String::from("d")),
            color_r: Some(String::from("r")),
            color_g: Some(String::from("g")),
            color_b: Some(String::from("b")),
            color_chooser: Some(String::from("c")),
            undo: Some(String::from("z")),
            clear_all: Some(String::from("x")),
        }
    }
}

impl Configuration {
    pub fn merge(self, other_config: Self) -> Self {
        Configuration {
            line_thickness: self.line_thickness.or(other_config.line_thickness),
            draw_keybind: self.draw_keybind.or(other_config.draw_keybind),
            arrow_keybind: self.arrow_keybind.or(other_config.arrow_keybind),
            reverse_arrow_keybind: self
                .reverse_arrow_keybind
                .or(other_config.reverse_arrow_keybind),
            rectangle_keybind: self.rectangle_keybind.or(other_config.rectangle_keybind),
            text_keybind: self.text_keybind.or(other_config.text_keybind),
            highlighter_keybind: self
                .highlighter_keybind
                .or(other_config.highlighter_keybind),
            disable_drawing: self.disable_drawing.or(other_config.disable_drawing),
            color_r: self.color_r.or(other_config.color_r),
            color_g: self.color_g.or(other_config.color_g),
            color_b: self.color_b.or(other_config.color_b),
            color_chooser: self.color_chooser.or(other_config.color_chooser),
            undo: self.undo.or(other_config.undo),
            clear_all: self.clear_all.or(other_config.clear_all),
        }
    }
}

pub const PENCIL_CUR: &str = "pencil";
pub const ARROW_CUR: &str = "arrow";
pub const SQUARE_CUR: &str = "rectangle";
pub const TEXT_CUR: &str = "text";
pub const HIGHLIGHTER_CUR: &str = "highlighter";

const CONFIG_NAME: &str = "chicolli.json";
const CONFIG_DIR: &str = "chicolli";
const CONFIG_CURSORS_DIR: &str = "cursors";

fn write_default_config(path: &std::path::Path) {
    let mut file = std::fs::File::create(path).unwrap();
    serde_json::to_writer_pretty(&mut file, &Configuration::default()).unwrap();
}

pub fn get_config() -> Configuration {
    match read_config() {
        Ok(conf) => conf,
        Err(r) => {
            eprintln!(
                "could not create the default config file, using default build in, {}",
                r
            );
            Configuration::default()
        }
    }
}

pub fn get_cursors_config_loc() -> Option<std::path::PathBuf> {
    let conf_path = config_dir();
    match conf_path {
        Some(mut conf_path) => {
            conf_path.push(CONFIG_DIR);
            conf_path.push(CONFIG_CURSORS_DIR);
            Some(conf_path)
        }
        None => None,
    }
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
                    write_default_config(conf_path.as_path());
                    read_config()
                }
            } else {
                std::fs::create_dir_all(conf_path.as_path())?;
                conf_path.push(CONFIG_NAME);
                write_default_config(conf_path.as_path());
                read_config()
            }
        }
        None => Err(Error::other("counld not find defaul config directory")),
    }
}

fn read_config_file(file_path: &std::path::Path) -> Result<Configuration, Error> {
    let mut file = File::open(file_path)?;

    // Read the content of the file into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Deserialize the JSON content into the Configuration struct
    let config = serde_json::from_str::<Configuration>(&content)?;

    Ok(config.merge(Configuration::default()))
}

#[cfg(test)]
mod tests {
    use super::Configuration;

    #[test]
    fn merge_prefers_present_values_and_falls_back_to_defaults_source() {
        let primary = Configuration {
            line_thickness: Some(9.0),
            draw_keybind: None,
            arrow_keybind: Some("a".to_string()),
            reverse_arrow_keybind: None,
            rectangle_keybind: None,
            text_keybind: Some("t".to_string()),
            highlighter_keybind: None,
            disable_drawing: None,
            color_r: Some("R".to_string()),
            color_g: None,
            color_b: None,
            color_chooser: None,
            undo: Some("u".to_string()),
            clear_all: None,
        };

        let fallback = Configuration::default();
        let merged = primary.merge(fallback);

        assert_eq!(merged.line_thickness, Some(9.0));
        assert_eq!(merged.arrow_keybind, Some("a".to_string()));
        assert_eq!(merged.text_keybind, Some("t".to_string()));
        assert_eq!(merged.color_r, Some("R".to_string()));
        assert_eq!(merged.undo, Some("u".to_string()));
        assert_eq!(merged.draw_keybind, Some("1".to_string()));
        assert_eq!(merged.reverse_arrow_keybind, Some("3".to_string()));
        assert_eq!(merged.rectangle_keybind, Some("4".to_string()));
        assert_eq!(merged.highlighter_keybind, Some("6".to_string()));
        assert_eq!(merged.disable_drawing, Some("d".to_string()));
        assert_eq!(merged.color_g, Some("g".to_string()));
        assert_eq!(merged.color_b, Some("b".to_string()));
        assert_eq!(merged.color_chooser, Some("c".to_string()));
        assert_eq!(merged.clear_all, Some("x".to_string()));
    }
}
