{
  description = "Rust Development Environment";

  inputs = {
    nixpkgs.url = "path:/nix/store/vp1i814m5cqfzh22bl76s1k9bc8iikkv-source";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }@inputs:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
    in
    {
      devShells.${system}.default = pkgs.mkShellNoCC {
        name = "rust-dev";

        buildInputs = [
          (pkgs.rust-bin.stable.latest.minimal.override {
            extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          })
          pkgs.pkg-config
          pkgs.openssl
        ];

        shellHook = ''
          echo "🚀 Rust development environment ready!"
          echo "Nixpkgs: ${pkgs.lib.version}"
          zeditor
        '';
      };
    };
}
