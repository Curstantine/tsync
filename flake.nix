{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      flake-utils,
      naersk,
      rust-overlay,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        naersk' = pkgs.callPackage naersk { };

        # Runtime dependencies
        buildInputs = with pkgs; [
          android-tools
          opusTools
        ];

        # Build-time dependencies
        nativeBuildInputs = with pkgs; [ ];
      in
      rec {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          src = ./.;
          bulidInputs = buildInputs;
          nativeBuildInputs = nativeBuildInputs;
        };

        # For `nix develop` (optional, can be skipped):
        devShell = pkgs.mkShell {
          nativeBuildInputs =
            with pkgs;
            [
              nixfmt-rfc-style
              nil
              rust-bin.stable.latest.default
            ]
            ++ buildInputs
            ++ nativeBuildInputs;
        };
      }
    );
}
