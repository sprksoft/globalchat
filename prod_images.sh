# Build the production docker image of the service and push to the server

set -e

RUST_IMAGES=( smppgc )

RUSTTARGET="armv7-unknown-linux-musleabihf"
DOCKERTARGET="linux/arm/v7"
IMAGES_DIR="prodimages"
PROJECT="smppserver"

# program override
CARGO="cargo"
GREP="grep"
RUSTUP="rustup"
SSH="ssh"
DOCKER="docker"
PROD_SERVER_DOCKER="sudo docker"
ESBUILD="esbuild"


setup ()
{
  $RUSTUP target add $RUSTTARGET
  $CARGO install cross

  if ! type esbuild ; then
    echo "ERROR: esbuild not found. You need to install esbuild."
    exit 1
  fi

  if ! groups | $GREP docker > /dev/null ; then
    sudo usermod -a -G docker $(whoami)
    echo "NOTE: user $(whoami) has been added to the docker group. Remove when done if you don't want this (sudo usermod -r -G docker $(whoami))"
    echo "LOG OUT AND BACK IN TO COMPLETE SETUP"
  fi
}

build ()
{
  echo "NOTE: you need to run $0 setup when it is the first time running this script"


  export SQLX_OFFLINE=true
  export RUSTUP_TOOLCHAIN=stable
  export RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

  for image in "${RUST_IMAGES[@]}" ; do

    echo "Building $image using cross..."
    cross build --target $RUSTTARGET --release --bin $image

    mkdir -p $IMAGES_DIR
    mkdir -p $IMAGES_DIR/$image
    cp -f $image/Rocket.toml $IMAGES_DIR/$image/Rocket.toml
    cp -rf $image/templates/* $IMAGES_DIR/$image/templates || true
    cp -rf $image/www/* $IMAGES_DIR/$image/www || true

    if ls $image/client > /dev/null ; then
      $ESBUILD --bundle --minify --sourcemap --outdir=$IMAGES_DIR/$image/www/ $image/client/v1.js $image/client/admin.js
    fi

    cp ./target/$RUSTTARGET/release/$image $IMAGES_DIR/$image/app
    $DOCKER buildx build --platform $DOCKERTARGET --build-arg APP=./$IMAGES_DIR/$image -t "${PROJECT}_$image:prod" -f prod.Dockerfile .

  done
}

push ()
{
  for image in ${RUST_IMAGES[@]} ; do
    echo "Sending $image image to $1..."
    $DOCKER save ${PROJECT}_$image:prod | $SSH $1 $PROD_SERVER_DOCKER load
  done
}

USAGE="Usage: $0 <cmd>
  setup
    install required tools
  build
    build images
  push username@host
    push the build production images to the server
"
case "$1" in
  setup)
    setup
    ;;
  build)
    build
    ;;
  push)
    push $2
    ;;
  *)
    echo "invalid cmd: $1"
    echo $USAGE
    exit 1
    ;;
esac
