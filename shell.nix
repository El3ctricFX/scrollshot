{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Build toolchain
    rustc
    cargo
    rust-analyzer
    rustfmt
    clippy
    nodejs_22
    pkg-config
    openssl

    # Tauri Linux dependencies
    webkitgtk_4_1
    libsoup_3
    gtk3
    glib
    cairo
    pango
    gdk-pixbuf
    harfbuzz
    librsvg

    # Runtime dependencies (this app shells out to these)
    grim
    slurp
    wtype
    ydotool
    wl-clipboard
  ];

  shellHook = ''
    export WEBKIT_DISABLE_DMABUF_RENDERER=1
    export WEBKIT_DISABLE_COMPOSITING_MODE=1
  '';
}