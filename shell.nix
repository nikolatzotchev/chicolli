{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    pkg-config
    rustc
    stdenv.cc
  ];

  buildInputs = with pkgs; [
    cairo
    gdk-pixbuf
    glib
    graphene
    gtk4
    gtk4-layer-shell
    libxkbcommon
    pango
    wayland
  ];
}
