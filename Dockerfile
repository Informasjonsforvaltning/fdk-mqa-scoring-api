FROM rust:latest AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    cmake

COPY ./ ./
RUN cargo build --release


FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/* \
    && curl -LO "http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2_amd64.deb" \
    && dpkg -i libssl1.1_1.1.1f-1ubuntu2_amd64.deb

ENV TZ=Europe/Oslo
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

COPY --from=builder /build/target/release/fdk-mqa-scoring-api /fdk-mqa-scoring-api

EXPOSE 8080
CMD ["/fdk-mqa-scoring-api"]
