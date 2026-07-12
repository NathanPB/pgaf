{
  description = "Crate's Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-gdal38.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, nixpkgs-gdal38, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { 
          inherit system; 
          overlays = [ rust-overlay.overlays.default ];
        };
        pkgs-gdal = import nixpkgs-gdal38 { inherit system; };
        
        rustToolchain = pkgs.rust-bin.nightly.latest.default;
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
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
