{ self, ... }: {
  perSystem = { system, pkgs, ... }:
    let
      rust = pkgs.fenix.stable;
      rustToolchain = pkgs.fenix.combine [
        (rust.withComponents [
          "cargo"
          "clippy"
          "rustc"
        ])
        pkgs.fenix.latest.rustfmt
        pkgs.fenix.latest.rust-analyzer
      ];
    in
    {
      devShells = {
        default = pkgs.mkShell {
          packages = with pkgs; [
            nixd
            natscli
            nats-server
            google-cloud-sdk
            cargo-make
            pkg-config
            opentofu
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
