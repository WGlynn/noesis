# Noesis testnet node — one self-contained binary that serves its own wallet UI + JSON API.
# Multi-stage: build the release `noesisd` on a full Rust image, then run it on a slim Debian.
# The frontend (frontend/index.html) is include_str!'d into the binary at compile time, so the
# builder MUST see it in context (do NOT .dockerignore it). The chain log lives on a mounted
# volume at /data so the chain survives machine restarts (durable, per store::load_chain).

# ---- build ----
FROM rust:1-bookworm AS build
WORKDIR /app
# Copy the whole workspace (build context; target/ and .git are excluded via .dockerignore).
COPY . .
# Build only the node binary. The RISC-V on-VM crates are workspace-excluded and not built here.
RUN cargo build --release -p noesis --bin noesisd

# ---- runtime ----
FROM debian:bookworm-slim
# curl is only for the fly healthcheck / manual probing; ca-certificates is good hygiene.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/noesisd /usr/local/bin/noesisd
# testnet genesis: distinct chain_id, PoW in consensus at low difficulty (worthless test JUL by construction).
ENV NOESIS_NET=testnet
# The durable chain log lives on the mounted volume.
VOLUME ["/data"]
EXPOSE 9955
# Bind all interfaces so fly's proxy can reach it; persist to the volume so a restart resumes the chain.
CMD ["noesisd", "--serve-api", "0.0.0.0:9955", "/data/chain.log"]
