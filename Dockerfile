FROM rust:latest AS builder

RUN update-ca-certificates

WORKDIR /app

COPY ./src src/
COPY Cargo.lock Cargo.lock 
COPY Cargo.toml Cargo.toml 

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12

LABEL "org.opencontainers.image.title"="camera-hook"

WORKDIR /app

COPY --from=builder /app/target/release/camera ./

USER 10001:10001

CMD ["/app/camera"]
