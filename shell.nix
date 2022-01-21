let
  # Pinned nixpkgs, deterministic. Last updated: 2/12/21.
  # pkgs = import (fetchTarball("https://github.com/NixOS/nixpkgs/archive/a58a0b5098f0c2a389ee70eb69422a052982d990.tar.gz")) {};

  # Rolling updates, not deterministic.
  pkgs = import (fetchTarball("channel:nixpkgs-unstable")) {};
in pkgs.mkShell {
  buildInputs = [ 
    pkgs.cargo 
    pkgs.clippy
    pkgs.rustc
    pkgs.rustfmt
    pkgs.xorg.libX11
    pkgs.vulkan-tools
    pkgs.glslang # or shaderc
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-validation-layers

    pkgs.libGL
    pkgs.libglvnd
    pkgs.mesa
    #pkgs.renderdoc
  ];

  LD_LIBRARY_PATH = with pkgs.xlibs; "${pkgs.mesa}/lib:${libX11}/lib:${libXcursor}/lib:${libXxf86vm}/lib:${libXi}/lib:${pkgs.xorg.libXrandr}/lib:${pkgs.libGL}/lib:/run/opengl-driver/lib:/run/opengl-driver-32/lib";

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}
