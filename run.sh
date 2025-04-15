set -e
DOCKER="docker"
DOCKER_COMPOSE="docker compose"

if ! $DOCKER version >> /dev/null ; then
  echo "docker possibly not installed or sudo canceled"
  exit 0
fi

if ! $DOCKER_COMPOSE version ; then
  echo "docker compose possibly not installed or sudo canceled"
  exit 0
fi

$DOCKER_COMPOSE -f compose.yml watch
