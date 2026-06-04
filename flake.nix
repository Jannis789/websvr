{
  description = "Rust Coding Env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
          ripgrep
          git
          zlib
          pkg-config
          openssl
          dprint
        ];

        shellHook = ''
          echo "🚀 Rust Shell geladen"
        '';
      };
    };
}
