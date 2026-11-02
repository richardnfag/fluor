# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.99-slim-trixie AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates clang mold pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install --locked cargo-chef
RUN rustup target add wasm32-wasip2
WORKDIR /app
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
ENV CARGO_TARGET_DIR=/app/target

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY . /app
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --locked --release && \
    cp target/release/api /app/api-binary && \
    RUSTFLAGS= cargo build --manifest-path modules/healthz/Cargo.toml --target wasm32-wasip2 --locked --release && \
    cp target/wasm32-wasip2/release/healthz.wasm /app/healthz.wasm

FROM chef AS dev
RUN cargo install --locked cargo-watch
CMD ["cargo", "watch", "-x", "run"]

FROM docker.io/library/debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system fluor \
    && useradd --system --gid fluor --home-dir /app --create-home fluor \
    && mkdir -p /data/db /data/wasm \
    && chown -R fluor:fluor /app /data
WORKDIR /app
COPY --from=builder /app/api-binary /usr/local/bin/fluor-api
COPY --from=builder /app/healthz.wasm /app/healthz.wasm
ENV DATABASE_URL=sqlite:///data/db/fluor.db
ENV WASM_STORAGE_PATH=/data/wasm
ENV HEALTHZ_WASM_PATH=/app/healthz.wasm
ENV HOST=0.0.0.0
ENV PORT=3001
USER fluor
EXPOSE 3001
CMD ["fluor-api"]
