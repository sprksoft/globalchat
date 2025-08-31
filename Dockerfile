FROM --platform=$BUILDPLATFORM tonistiigi/xx AS xx

FROM --platform=$BUILDPLATFORM rust:alpine AS builder
  COPY --from=xx / /
  RUN apk update && apk add clang pkgconfig ca-certificates lld
  ARG TARGETPLATFORM
  RUN xx-apk update && xx-apk add gcc musl musl-dev openssl-dev openssl-libs-static
  RUN xx-clang --setup-target-triple

  ENV RUSTUP_TOOLCHAIN=stable
  ENV SQLX_OFFLINE=true

  COPY . /build

  ARG BINARY=smppgc
  ARG RELEASE
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

export OPENSSL_DIR="/$(xx-info triple)/usr"
if ! xx-info is-cross ; then
  export OPENSSL_DIR="/usr"
fi
export OPENSSL_STATIC=1
export CC="xx-clang"

mkdir /app
xx-cargo build $REL_ARG --bin $BINARY
cp target/$(xx-cargo --print-target-triple)/$PROFILE/$BINARY /app/app
xx-verify /app/app --static
EOF


############## SMPPGC ##############

FROM builder AS dev
  RUN apk update && apk add typescript esbuild
  COPY smppgc/Rocket.toml /app/Rocket.toml
  COPY smppgc/templates /app/templates
  COPY smppgc/www /app/www
  COPY smppgc/client /client

  WORKDIR /client
  RUN esbuild --outdir=/app/www $(cat esbuild_cmd)

  COPY --chmod=777 <<EOF /entry.sh
#!/bin/sh
#nohup tsc --noEmit --watch &
nohup esbuild --outdir=/app/www --watch=forever $(cat esbuild_cmd) &
cd /app
exec /app/app
EOF

  EXPOSE 8080

  CMD [ "/entry.sh" ]

FROM scratch AS prod
  COPY --from=dev /app /app
  COPY --from=dev /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

  EXPOSE 8080

  WORKDIR /app
  CMD [ "/app/app" ]
