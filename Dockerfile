ARG ESBUILD_CMD="esbuild --bundle --minify --sourcemap --outdir=/app/www/ /client/chat.js /client/home.js /client/login.js /client/mods.js"
ARG BINARY_SOURCE="builder"

FROM rust:alpine AS builder
  RUN apk update && apk add esbuild musl-dev

  ENV SQLX_OFFLINE=true
  ENV RUSTUP_TOOLCHAIN=stable

  COPY . /build
  WORKDIR /build

  RUN --mount=type=cache,target=target/ \
      --mount=type=cache,target=/usr/local/cargo/registry/ \
      --mount=type=cache,target=/usr/local/rustup/ \
      <<EOF
set -e
cargo build --locked --release --bin smppgc
mkdir /app

cp target/release/smppgc /app/app
EOF

FROM rust:alpine AS artifact
  RUN apk update && apk add esbuild
  COPY ./.artifacts/smppgc /app/app

FROM $BINARY_SOURCE AS late-builder
  ARG ESBUILD_CMD
  COPY smppgc/Rocket.toml /app/Rocket.toml
  COPY smppgc/templates /app/templates
  COPY smppgc/www /app/www

  COPY smppgc/client /client
  RUN $ESBUILD_CMD

FROM late-builder AS dev
  ARG ESBUILD_CMD

  WORKDIR /app

  EXPOSE 8080

  ENV ROCKET_CONFIG=/app/Rocket.toml
  ENV ROCKET_PROFILE=debug

  COPY --chmod=777 <<EOF /entry.sh
#!/bin/sh
cd /build
nohup $ESBUILD_CMD --watch=forever &
cd /app
exec /app/app
EOF

  CMD [ "/entry.sh" ]


FROM scratch AS prod
  COPY --from=late-builder /app /app

  EXPOSE 8080
  WORKDIR /app
  ENV ROCKET_CONFIG=/app/Rocket.toml
  ENV ROCKET_PROFILE=release
  CMD [ "/app/app" ]
