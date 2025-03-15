ARG RUST_VERSION="1.83"
ARG RUST_IMAGE="rust:${RUST_VERSION}-bullseye"
ARG BASE_IMAGE="debian:bullseye"

FROM ${RUST_IMAGE} as install

RUN apt-get -y update && apt-get -y upgrade
# Install cargo chef
RUN cargo install cargo-chef --locked

WORKDIR /app

# -- Cache for builder --
FROM install as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -- Builder with cache layer --
FROM install as builder
COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --workspace --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --workspace

# -- Base Image --
FROM ${BASE_IMAGE} as base
RUN apt-get update && apt-get install -y ca-certificates

# -- Production Image --
FROM base as prod

WORKDIR /app
COPY --from=builder /app/target/release/grafana-shogun ./grafana-shogun
RUN ldconfig
COPY ./scripts ./scripts
COPY ./config.toml ./config.toml

ENTRYPOINT ["/app/scripts/entry.sh"]