# syntax=docker/dockerfile:1

FROM docker.io/library/rust:1.92-slim-trixie AS chef
RUN apt-get update && apt-get install -y clang mold && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
WORKDIR /app/api
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/api/recipe.json recipe.json
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY . /app
WORKDIR /app/api
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/api /app/api-binary

FROM chef AS dev
RUN cargo install cargo-watch
WORKDIR /app/api
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"
CMD ["cargo", "watch", "-x", "run"]

FROM docker.io/library/debian:trixie-slim AS runtime
WORKDIR /usr/local/bin
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/api-binary ./fluor-api
EXPOSE 3001
CMD ["./fluor-api"]