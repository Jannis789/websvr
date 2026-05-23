{
  description = "Rust + Aider + Z.AI Coding Env";

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
          aider-chat
          git
        ];

        shellHook = ''
          echo "🚀 Rust + Aider + Z.AI Shell geladen"

          # ❗ WICHTIG: richtiger Coding Endpoint
          export OPENAI_API_BASE="https://api.z.ai/api/coding/paas/v4"

          # API Key aus Datei laden
          if [ -f .api_key ]; then
            export OPENAI_API_KEY=$(cat .api_key)
            echo "🔑 API Key geladen"
          else
            echo "❌ .api_key fehlt!"
          fi

          echo "👉 Nutze: aider --model openai/glm-5.1"
        '';
      };
    };
}
