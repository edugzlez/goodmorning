FROM rust:1-bullseye AS planner
WORKDIR /app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1-bullseye AS cacher
WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN apt update && apt install --no-install-recommends -y libssl-dev pkg-config && apt clean
RUN OPENSSL_STATIC=1 cargo chef cook --release --recipe-path recipe.json

FROM rust:1-bullseye AS builder
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release --bin goodmorning

FROM debian:bullseye-slim AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/goodmorning /usr/local/bin/
# RUN apt update && apt install --no-install-recommends -y openssl && apt clean
ENTRYPOINT ["goodmorning"]
