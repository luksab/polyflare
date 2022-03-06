let
  # Pinned nixpkgs, deterministic. Last updated: 2/12/21.
  pkgs = import (fetchTarball
    ("https://github.com/NixOS/nixpkgs/archive/f0e0efabb0dbe2f68d02c1a84cf6074bb8539016.tar.gz"))
    { };
  my-python-packages = python-packages:
    with python-packages; [
      numpy
      scipy
      matplotlib
      # other python packages you want
    ];
  python-with-my-packages = pkgs.python3.withPackages my-python-packages;

  # Rolling updates, not deterministic.
  #   pkgs = import (fetchTarball ("channel:nixpkgs-unstable")) { };
in pkgs.mkShell {
  buildInputs = with pkgs; [
    pandoc
    gnuplot
    python-with-my-packages
    imagemagick
  ];
}
