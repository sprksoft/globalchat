set -e
DOCKER="docker"
DOCKER_COMPOSE="docker compose"
CARGO="cargo"

if ! $DOCKER version >> /dev/null ; then
  echo "docker possibly not installed or sudo canceled"
  exit 1
fi

if ! $DOCKER_COMPOSE version ; then
  echo "docker compose possibly not installed or sudo canceled"
  exit 1
fi


if [[ "$1" == "prepare" ]] ; then
  if ! type $CARGO ; then
    echo "Cargo not found"
    exit 1
  fi
  $DOCKER_COMPOSE -f db.compose.yml up --detach

  echo "sqlx prepare..."
  $CARGO sqlx prepare --workspace
  exit 0
fi

$DOCKER_COMPOSE -f compose.yml watch
