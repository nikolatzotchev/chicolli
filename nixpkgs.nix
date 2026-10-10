# Pinned nixpkgs (nixos-26.05: rustc 1.95, GTK 4.22, gtk4-layer-shell 1.3) so the
# Nix shell and package build the same way until this pin is updated on purpose.
# To update, point `url` at a newer channel's nixexprs.tar.xz and refresh `sha256`
# with `nix-prefetch-url --unpack <url>`.
import (fetchTarball {
  url = "https://releases.nixos.org/nixos/26.05/nixos-26.05.11576.7c8764b7c7b0/nixexprs.tar.xz";
  sha256 = "1fzrxmr2qi7fy76wh9v0l1a7vdvscxn4p2d5ng9rhpcnrhdil6zy";
})
