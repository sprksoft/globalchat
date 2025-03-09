DOCKER_COMPOSE="sudo docker compose"

$DOCKER_COMPOSE -f compose.yml -f prod.compose.yml up --detach || exit 1
echo "done"
