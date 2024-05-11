FROM rust:1-slim-buster AS build

RUN cargo new --bin app
WORKDIR /app
RUN cargo new --bin server

COPY server/Cargo.toml /app/
COPY Cargo.lock /app/
RUN cargo build --release

COPY server/src /app/src
RUN touch /app/src/main.rs
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /app/target/release/server /app/server

CMD "/app/server"