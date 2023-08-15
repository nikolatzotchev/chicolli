# chicolli
Gtk4 shell drawing tool for wayland.
**WIP** as of right now.

# Installation
### gtk4-layer-shell
You will need [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell)
(At the time of writing, it is only available in a very limited number of distros, so you will most likely have to build it from source, one can check their readme for an example in ubunto, and the worflow of this repository for an example in fedora)

# Configuration

The configuration file is placed in `$XDG_CONFIG_HOME/chicolli/chicolli.json` or `$HOME/.config/chicolli.json` the rust crate [dirs](https://crates.io/crates/dirs) is used for finding it.

More information can be found in [Configuration](https://github.com/nikolatzotchev/chicolli/wiki/Configuration).
