with import <nixpkgs> {
  overlays = map (uri: import (fetchTarball uri))
    [ "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz" ];
};

pkgs.mkShell {
  buildInputs = [
    pkgs.xorg.libX11
    pkgs.vulkan-tools
    pkgs.glslang # or shaderc
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-validation-layers

    pkgs.libGL
    pkgs.libglvnd
    pkgs.mesa
    pkgs.renderdoc

    cargo-web
    (latest.rustChannels.nightly.rust.override {
      targets = [ "wasm32-unknown-unknown" ];
    })
  ];

  LD_LIBRARY_PATH = with pkgs.xlibs; "${pkgs.mesa}/lib:${libX11}/lib:${libXcursor}/lib:${libXxf86vm}/lib:${libXi}/lib:${pkgs.xorg.libXrandr}/lib:${pkgs.libGL}/lib:/run/opengl-driver/lib:/run/opengl-driver-32/lib";
}
