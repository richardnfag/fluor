FROM rust:latest AS builder

WORKDIR /usr/src/function/

RUN rustup target add x86_64-unknown-linux-musl
ADD source.tar.gz /usr/src/function/
RUN cargo build --release --target x86_64-unknown-linux-musl

RUN NAME=$(awk '/name/ {print $3}' Cargo.toml | sed 's/[[:punct:]]//g') && \
    mv target/x86_64-unknown-linux-musl/release/$NAME ./function


FROM scratch AS runtime

COPY --from=builder /usr/src/function/function /usr/local/bin/

CMD [ "/usr/local/bin/function" ]