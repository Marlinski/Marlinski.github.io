# ─── minitel: marlinski.org over SSH ──────────────────────────────────────────
#
#   docker build -t minitel minitel/
#   docker run --rm -p 2222:2222 -v minitel-data:/data minitel
#   ssh -p 2222 localhost
#
# Build context is minitel/, not the repo root: the service carries no content,
# it reads marlinski.org/minitel.json at runtime.

FROM rust:1-alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /src
# Dependencies first, so editing src/ does not refetch and rebuild the world.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
 && cargo build --release \
 && rm -rf src

COPY src/ src/
# cargo skips a rebuild if mtimes look unchanged; touch to force it.
RUN touch src/main.rs && cargo build --release

# ─── runtime ─────────────────────────────────────────────────────────────────
FROM alpine:3.21

LABEL org.opencontainers.image.source="https://github.com/Marlinski/Marlinski.github.io"
LABEL org.opencontainers.image.description="marlinski.org served over SSH, Minitel style"

RUN apk add --no-cache ca-certificates \
 && adduser -D -u 1000 minitel \
 && mkdir -p /data && chown minitel:minitel /data

COPY --from=build /src/target/release/minitel /usr/local/bin/minitel

USER minitel
# The host key lives here so it survives restarts; without a volume every
# reconnect trips the client's known_hosts warning.
VOLUME ["/data"]
EXPOSE 2222

ENV MINITEL_PORT=2222 \
    MINITEL_HOST_KEY=/data/host_key \
    MINITEL_FEED=https://marlinski.org/minitel.json

ENTRYPOINT ["minitel"]
