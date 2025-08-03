FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx

FROM --platform=$BUILDPLATFORM rust:alpine AS builder
  COPY --from=xx / /
  RUN apk update && apk add esbuild musl-dev clang lld perl make

  ENV RUSTUP_TOOLCHAIN=stable
  ENV SQLX_OFFLINE=true

  COPY Cargo.toml build/
  COPY Cargo.lock build/
  COPY .sqlx build/.sqlx

  COPY lmetrics build/lmetrics/
  COPY profanity build/profanity/
  COPY string_tree build/string_tree/
  COPY wordfilter build/wordfilter/

  ARG BINARY=smppgc
  COPY $BINARY/migrations build/$BINARY/migrations
  COPY $BINARY/src build/$BINARY/src/
  COPY $BINARY/Cargo.toml build/$BINARY/

  ARG RELEASE
  ARG TARGETPLATFORM
  RUN --mount=type=cache,target=/build/target \
      --mount=type=cache,sharing=locked,target=/usr/local/cargo/registry/ \
      --mount=type=cache,sharing=locked,target=/usr/local/rustup/ \
      <<EOF
set -e
cd /build

REL_ARG="--release"
PROFILE="release"
if [[ "$RELEASE" != "true" ]]
then
  REL_ARG=""
  PROFILE="debug"
fi

mkdir /app
xx-cargo build $REL_ARG --locked --bin $BINARY
cp target/$(xx-cargo --print-target-triple)/$PROFILE/$BINARY /app/app
xx-verify /app/app
EOF


FROM builder AS dev
  COPY smppgc/Rocket.toml /app/Rocket.toml
  COPY smppgc/templates /app/templates
  COPY smppgc/www /app/www
  COPY smppgc/client /client


  EXPOSE 8080

  WORKDIR /client
  RUN esbuild --outdir=/app/www $(cat esbuild_cmd)

  COPY --chmod=777 <<EOF /entry.sh
#!/bin/sh
nohup esbuild --outdir=/app/www --watch=forever $(cat esbuild_cmd) &
cd /app
exec /app/app
EOF

  CMD [ "/entry.sh" ]

FROM scratch AS prod
  COPY --from=dev /app /app
  COPY --from=dev /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

  EXPOSE 8080
  WORKDIR /app
  CMD [ "/app/app" ]
