# Pinned build environment for the Containerfile build stage and local dev.
# Enter: nix-shell    Build: nix-shell --run 'cargo build --release --locked'
let
  nixpkgs = fetchTarball {
    # nixos-26.05, 2026-11-08
    url = "https://github.com/NixOS/nixpkgs/archive/ac588ef1c914865dad517b337ea0eb9694a4021c.tar.gz";
    sha256 = "1sxd2sqlpy1bx6b5xiwy1pbyygazl9d1a5yy0rvqfsihqpcibq2y";
  };
  rustOverlay = fetchTarball {
    # oxalica/rust-overlay master, 2026-11-12
    url = "https://github.com/oxalica/rust-overlay/archive/8bbfc858c7b85f5e6ed6d6cc201ffcee1bb08a9b.tar.gz";
    sha256 = "0dk7kbba29zgdn1ydsr59ipfbn1rjwljcs73cclg0zwl2rninv1n";
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
