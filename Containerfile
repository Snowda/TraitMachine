# syntax=docker/dockerfile:1
# Multi-stage build for the traitmachine demo CLI.
# Build:  podman build -t traitmachine -f Containerfile .
# Run:    podman run --rm traitmachine

# ---- build stage ----
FROM docker.io/library/rust:1.96-bookworm AS build
WORKDIR /src

# Cache dependency compilation separately from source changes.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the real sources.
COPY . .
RUN cargo build --release --locked --bin traitmachine

# ---- runtime stage ----
FROM docker.io/library/debian:bookworm-slim AS runtime

# Run as an unprivileged user.
RUN useradd --create-home --uid 10001 app
USER app

COPY --from=build /src/target/release/traitmachine /usr/local/bin/traitmachine

ENTRYPOINT ["/usr/local/bin/traitmachine"]
