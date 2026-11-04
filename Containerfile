# syntax=docker/dockerfile:1
# Multi-stage build for the traitmachine demo CLI.
# Build:  podman build -t traitmachine -f Containerfile .
# Run:    podman run --rm traitmachine

# ---- build stage ----
FROM docker.io/library/rust:1.99.1-bookworm AS build
WORKDIR /src

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the real sources.
COPY . .
RUN cargo build --release --locked --bin traitmachine \
    && mkdir -p /rootfs/home/app

# ---- runtime stage: distroless/cc (glibc + libgcc), no shell ----
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

# Run as an unprivileged user; the home dir is made in the build stage.
COPY --from=build --chown=10001:10001 /rootfs/home/app /home/app
ENV HOME=/home/app
# distroless:nonroot sets WORKDIR /home/nonroot; keep the previous cwd.
WORKDIR /
USER 10001:10001

COPY --from=build /src/target/release/traitmachine /usr/local/bin/traitmachine

ENTRYPOINT ["/usr/local/bin/traitmachine"]
