# Stockfront API (stockfront-app). Built by docker-compose.yml as the `backend` service.

FROM rust:1.98-slim-trixie AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# The cache mounts keep crates and incremental output between deploys; the
# binary is copied out in the same step because the target mount is not
# part of the image.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p stockfront-app \
    && cp target/release/stockfront-app /usr/local/bin/stockfront-app

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home stockfront
COPY --from=build /usr/local/bin/stockfront-app /usr/local/bin/stockfront-app
USER stockfront
ENV STOCKFRONT_BIND_ADDRESS=0.0.0.0:3100
EXPOSE 3100
CMD ["stockfront-app"]
