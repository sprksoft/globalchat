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
  export RUSTFLAGS="-Clink-self-contained=yes"

  cross build --target $RUSTTARGET --release --locked
  mkdir -p .artifacts
  cp -rf target/$RUSTTARGET/release/smppgc .artifacts/smppgc # Copy artifacts because target/ is in .dockerignore

  $DOCKER buildx build --platform $DOCKERTARGET --build-arg BINARY_SOURCE=artifact -f smppgc/Dockerfile -t "smppserver_smppgc:prod" .

}

push_image () {
  echo "Sending $1 image to $2..."
  $DOCKER save $1 | $SSH $2 $PROD_SERVER_DOCKER load
}

push-deploy ()
{
  image="smppserver_smppgc:beta"
  push_image $image $1

  echo "redeploying service on prod server..."
  $SSH $PROD_SERVER "~/source/repos/ldeveuorg-infra/deploy.sh $image"
}

USAGE="Usage: $0 <cmd>
  setup
    install required tools
  build
    build images
  push-deploy username@host
    push the build production images to the server
"
case "$1" in
  setup)
    setup
    ;;
  build)
    build
    ;;
  push-deploy)
    push-deploy $2
    ;;
  *)
    echo "invalid cmd: $1"
    echo $USAGE
    exit 1
    ;;
esac
