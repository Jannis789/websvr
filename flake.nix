{
  description = "Rust development environment with rama";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.rustc
            pkgs.cargo
            pkgs.pkg-config
            pkgs.openssl
          ];
          shellHook = ''
            if ! cargo install --list | grep -q '^rama '; then
              echo "Adding Rama Franework..."
              cargo add rama --features http-full
              echo "Adding SeaORM with SQLite support..."
              cargo add sea-orm --features sqlx-sqlite,runtime-tokio-native-tls,macros
              echo "Adding Serde with derive feature..."
              cargo add serde --features derive
              echo "Adding Tokio with full feature set..."
              cargo add tokio --features full
              cargo add serde
            fi
          '';
        };
      }
    );
}
