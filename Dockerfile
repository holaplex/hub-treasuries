FROM rust:1.71.0-bullseye as chef
RUN cargo install cargo-chef --locked

WORKDIR /app

RUN apt-get update -y && \
  apt-get install -y --no-install-recommends \
    cmake \
    g++ \
    libsasl2-dev \
    libssl-dev \
    libudev-dev \
    wget \
    pkg-config \
  && \
  rm -rf /var/lib/apt/lists/*

COPY ci/get-protoc.sh ./
RUN chmod +x get-protoc.sh
RUN /app/get-protoc.sh

FROM chef AS planner
COPY Cargo.* ./
COPY api api
COPY migration migration
COPY fireblocks fireblocks
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY Cargo.* ./
COPY api api
COPY migration migration
COPY fireblocks fireblocks

FROM builder AS builder-hub-treasuries
RUN cargo build --release --bin holaplex-hub-treasuries

FROM builder AS builder-migration
RUN cargo build --release --bin migration

FROM debian:bullseye-slim as base
WORKDIR /app
RUN apt-get update -y && \
  apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    libssl1.1 \
  && \
  rm -rf /var/lib/apt/lists/*

RUN mkdir -p bin

FROM base AS hub-treasuries
ENV TZ=Etc/UTC
ENV APP_USER=runner

RUN groupadd $APP_USER \
    && useradd --uid 10000 -g $APP_USER $APP_USER \
    && mkdir -p bin

RUN chown -R $APP_USER:$APP_USER bin

USER 10000
COPY --from=builder-hub-treasuries /app/target/release/holaplex-hub-treasuries bin
CMD ["bin/holaplex-hub-treasuries"]

FROM base AS migrator
COPY --from=builder-migration /app/target/release/migration bin/
CMD ["bin/migration"]
