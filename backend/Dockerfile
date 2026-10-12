# syntax=docker/dockerfile:1
# Build context is the repository root (see compose.yaml).
FROM rust:1.99-slim-bookworm AS build
WORKDIR /app/backend
ENV CARGO_BUILD_JOBS=1
COPY catalog/sources.json /app/catalog/sources.json
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations
RUN cargo build --locked --release -j 1 && cp target/release/service-guide /service-guide

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=build /service-guide /usr/local/bin/service-guide
USER 65532:65532
EXPOSE 3001
# serve already applies sqlx's tracked, idempotent migrations; never indexes.
ENTRYPOINT ["service-guide"]
CMD ["serve"]
