use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Error, ErrorKind, Read},
};

use dirs::config_dir;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub line_thickness: Option<f64>,
    pub draw_keybind: Option<String>,
    pub arrow_keybind: Option<String>,
    pub reverse_arrow_keybind: Option<String>,
    pub rectangle_keybind: Option<String>,
    pub disable_drawing: Option<String>,
    pub color_r: Option<String>,
    pub color_g: Option<String>,
    pub color_b: Option<String>,
    pub color_chooser: Option<String>,
}

impl Configuration {
    fn minimal() -> Self {
        Configuration {
            line_thickness: Some(2.0),
            draw_keybind: None,
            arrow_keybind: None,
            reverse_arrow_keybind: None,
            rectangle_keybind: None,
            disable_drawing: None,
            color_r: None,
            color_g: None,
            color_b: None,
            color_chooser: None,
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            line_thickness: Some(2.0),
            draw_keybind: Some(String::from("1")),
            arrow_keybind: Some(String::from("2")),
            reverse_arrow_keybind: Some(String::from("3")),
            rectangle_keybind: Some(String::from("4")),
            disable_drawing: Some(String::from("d")),
            color_r: Some(String::from("r")),
            color_g: Some(String::from("g")),
            color_b: Some(String::from("b")),
            color_chooser: Some(String::from("c")),
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
            disable_drawing: self.disable_drawing.or(other_config.disable_drawing),
            color_r: self.color_r.or(other_config.color_r),
            color_g: self.color_g.or(other_config.color_g),
            color_b: self.color_b.or(other_config.color_b),
            color_chooser: self.color_chooser.or(other_config.color_chooser),
        }
    }
}

pub const PENCIL_CUR: &str = "pencil";
pub const ARROW_CUR: &str = "arrow";
pub const SQUARE_CUR: &str = "rectangle";

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
                    return Ok(config);
                } else {
                    write_default_config(conf_path.as_path());
                    return read_config();
                }
            } else {
                std::fs::create_dir_all(conf_path.as_path())?;
                conf_path.push(CONFIG_NAME);
                write_default_config(conf_path.as_path());
                return read_config();
            }
        }
        None => Err(Error::new(
            ErrorKind::Other,
            "counld not find defaul config directory",
        )),
    }
}

fn read_config_file(file_path: &std::path::Path) -> Result<Configuration, Error> {
    let mut file = File::open(file_path)?;

    // Read the content of the file into a string
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Deserialize the JSON content into the Configuration struct
    let config = serde_json::from_str::<Configuration>(&content)?;

    return Ok(config.merge(Configuration::minimal()));
}
