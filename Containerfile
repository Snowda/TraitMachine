# syntax=docker/dockerfile:1
# Multi-stage build for the traitmachine demo CLI.
# Build:  podman build -t traitmachine -f Containerfile .
# Run:    podman run --rm traitmachine

# ---- build stage: Nix supplies the toolchain pinned in shell.nix ----
FROM docker.io/nixos/nix:2.31.5 AS build
WORKDIR /src

# Realise the pinned toolchain in its own layer.
COPY shell.nix rust-toolchain.toml ./
RUN nix-shell --run 'cargo --version'

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && nix-shell --run 'cargo build --release' \
    && rm -rf src

# Build the real sources as a static musl binary.
COPY . .
RUN nix-shell --run 'cargo build --release --locked --bin traitmachine \
        && install -D "target/$CARGO_BUILD_TARGET/release/traitmachine" /rootfs/usr/local/bin/traitmachine' \
    && mkdir -p /rootfs/home/app

# ---- runtime stage: distroless/static (no libc needed), no shell ----
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

# Run as an unprivileged user; the home dir is made in the build stage.
COPY --from=build --chown=10001:10001 /rootfs/home/app /home/app
ENV HOME=/home/app
# distroless:nonroot sets WORKDIR /home/nonroot; keep the previous cwd.
WORKDIR /
USER 10001:10001

COPY --from=build /rootfs/usr/local/bin/traitmachine /usr/local/bin/traitmachine

ENTRYPOINT ["/usr/local/bin/traitmachine"]
