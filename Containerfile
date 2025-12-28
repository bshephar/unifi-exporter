FROM rust:latest AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev

WORKDIR /app
COPY . .
RUN cargo build --release

FROM fedora:latest

RUN microdnf install -y ca-certificates openssl && \
    microdnf clean all

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/unifi-exporter ./exporter

EXPOSE 8080

CMD ["./exporter"]
