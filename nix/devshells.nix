{ self, ... }: {
  perSystem = { system, pkgs, ... }:
    let
      rust = pkgs.fenix.latest;
      rustToolchain = pkgs.fenix.combine [
        (rust.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
          "rust-analyzer"
        ])
      ];
    in
    {
      devShells = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            nixd
            natscli
            nats-server
            cargo-make
            pkg-config
            rustToolchain
            vscode-extensions.vadimcn.vscode-lldb
          ];
          shellHook = ''
            export RUST_SRC_PATH="${rust.rust-src}/lib/rustlib/src/rust/library";
            export PATH=$HOME/.cargo/bin:$PATH
          '';
        };
      };
    };
}
