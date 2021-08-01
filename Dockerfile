FROM rust:buster as build

# Dependencies
RUN USER=root apt-get update && apt-get install -y
RUN USER=root apt-get install -y libudev-dev

# Create a new empty shell project
RUN USER=root cargo new --bin solana-exporter
WORKDIR /solana-exporter

# Copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build and cache deps
RUN cargo build --release
RUN rm src/*.rs

# Copy source
COPY ./src ./src

# Build for release
RUN rm ./target/release/deps/solana_exporter*
RUN cargo build --release

# Final base
FROM debian:buster

RUN USER=root apt-get update && apt-get install -y
RUN USER=root apt-get install -y libssl-dev libudev-dev

COPY --from=build /solana-exporter/target/release/solana-exporter .

RUN mkdir /etc/solana-exporter
ENV RUST_LOG=debug

CMD ["./solana-exporter", "-c",  "/etc/solana-exporter/config.toml", "-d" , "/exporter/persistent.db"]