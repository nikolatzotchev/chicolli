# Builds and installs Chicolli: `nix-build`, or `nix-env -if .` to install it.
{ pkgs ? import ./nixpkgs.nix { } }:

let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
pkgs.rustPlatform.buildRustPackage {
  pname = "chicolli";
  version = cargoToml.package.version;

  src = pkgs.lib.cleanSourceWith {
    src = ./.;
    filter = path: type: !(pkgs.lib.hasPrefix "target" (baseNameOf path)) && baseNameOf path != "result";
  };
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = with pkgs; [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = with pkgs; [
    gtk4
    gtk4-layer-shell
    wayland
  ];

  # The tests draw text with Pango, which needs fonts the build sandbox lacks.
  FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; };

  # grim, slurp and wl-copy are run for copy and save; put them on PATH.
  preFixup = ''
    gappsWrapperArgs+=(--suffix PATH : ${pkgs.lib.makeBinPath (with pkgs; [ grim slurp wl-clipboard ])})
  '';

  postInstall = ''
    install -Dm644 data/io.github.nikolatzotchev.Chicolli.desktop -t $out/share/applications
    install -Dm644 data/io.github.nikolatzotchev.Chicolli.svg -t $out/share/icons/hicolor/scalable/apps
  '';

  meta = {
    description = cargoToml.package.description;
    homepage = cargoToml.package.repository;
    license = pkgs.lib.licenses.mit;
    mainProgram = "chicolli";
    platforms = pkgs.lib.platforms.linux;
  };
}
