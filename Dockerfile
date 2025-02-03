FROM rust:latest AS builder
RUN apt-get update && apt-get install -y esbuild


ENV SQLX_OFFLINE=true
ENV RUSTUP_TOOLCHAIN=stable

COPY . /build
WORKDIR /build

RUN --mount=type=cache,target=target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=cache,target=/usr/local/rustup/ \
    <<EOF
set -e
extra_args="--release"
if [[ "$DEBUG" == "true" ]] ; then
  extra_args=""
fi
cargo build --locked $extra_args --bin smppgc
mkdir /app
cp target/release/smppgc /app/app
EOF


COPY smppgc/Rocket.toml /app/Rocket.toml
COPY smppgc/templates /app/templates
COPY smppgc/www /app/www

RUN esbuild smppgc/client/index.js --bundle --minify --sourcemap --outfile=/app/www/v1.js

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y


RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid 10001 \
    appuser

COPY --from=builder --chown=appuser:appuser /app /app

USER appuser
WORKDIR /app

EXPOSE 8080

ENV ROCKET_CONFIG=/app/Rocket.toml
CMD [ "/app/app" ]

