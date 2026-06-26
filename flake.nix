{
  description = "Crate's Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-gdal38.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-gdal38, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        pkgs-gdal = import nixpkgs-gdal38 { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
            rustPlatform.bindgenHook 
          ];

          buildInputs = [
            pkgs-gdal.gdal
          ];
          GDAL_CONFIG = "${pkgs.lib.getDev pkgs-gdal.gdal}/bin/gdal-config";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        };
      }
    );
}
