{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs =
    {
      self,
      flake-utils,
      naersk,
      nixpkgs,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

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
              rustc
              cargo
              clippy
              nixfmt-rfc-style
              nil
            ]
            ++ buildInputs
            ++ nativeBuildInputs;
        };
      }
    );
}
