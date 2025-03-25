FROM rust:alpine AS dev
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

COPY smppgc/Rocket.toml /app/Rocket.toml
COPY smppgc/templates /app/templates
COPY smppgc/www /app/www

ENV ESBUILD_CMD="esbuild --bundle --minify --sourcemap --outdir=/app/www/ smppgc/client/v1.js smppgc/client/admin.js"
RUN $ESBUILD_CMD

WORKDIR /app
STOPSIGNAL SIGINT
EXPOSE 8080

ENV ROCKET_CONFIG=/app/Rocket.toml
ENV ROCKET_PROFILE=debug

COPY --chmod=777 <<EOF /entry.sh
#!/bin/sh
cd /build
nohup $ESBUILD_CMD --watch=forever &
cd /app
/app/app
EOF

CMD [ "/entry.sh" ]


FROM scratch AS prod

COPY --from=dev /app /app

EXPOSE 8080
STOPSIGNAL SIGINT
WORKDIR /app
ENV ROCKET_CONFIG=/app/Rocket.toml
ENV ROCKET_PROFILE=release
CMD [ "/app/app" ]
