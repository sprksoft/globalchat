DOCKER="sudo docker"
DOCKER_COMPOSE="sudo docker compose"

if ! $DOCKER version >> /dev/null ; then
  echo "docker possibly not installed or sudo canceled"
  exit 0
fi

if ! $DOCKER_COMPOSE version ; then
  echo "docker compose possibly not installed or sudo canceled"
  exit 0
fi

if [[ $* == *--nodbgenv* ]] ; then
$DOCKER_COMPOSE -f compose.yml up || exit 1
else
$DOCKER_COMPOSE -f compose.yml -f debugenv.compose.yml watch || exit 1
fi

