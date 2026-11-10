# Pinned build environment for the Containerfile build stage and local dev.
# Enter: nix-shell    Build: nix-shell --run 'cargo build --release --locked'
let
  nixpkgs = fetchTarball {
    # nixos-26.05, 2026-11-08
    url = "https://github.com/NixOS/nixpkgs/archive/ac588ef1c914865dad517b337ea0eb9694a4021c.tar.gz";
    sha256 = "1sxd2sqlpy1bx6b5xiwy1pbyygazl9d1a5yy0rvqfsihqpcibq2y";
  };
  rustOverlay = fetchTarball {
    # oxalica/rust-overlay master, 2026-11-09
    url = "https://github.com/oxalica/rust-overlay/archive/b70538330f20adc9665afa2976471eb5b85ebd1f.tar.gz";
    sha256 = "0qraxcpbs2dg4k1nrmihrpnw3ws3h0mz975lzn6nkzswx7w2zvi7";
  };
  pkgs = import nixpkgs { overlays = [ (import rustOverlay) ]; };

  # Static musl binaries run on the distroless/static runtime without the Nix glibc.
  muslTarget = "${pkgs.stdenv.hostPlatform.parsed.cpu.name}-unknown-linux-musl";
  rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
    targets = [ muslTarget ];
  };
in
pkgs.mkShell {
  packages = [ rust ];
  CARGO_BUILD_TARGET = muslTarget;
}
