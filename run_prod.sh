set -e
DOCKER="sudo docker"
DOCKER_COMPOSE="sudo docker compose"
GIT="git"

$GIT reset --hard || exit 1
$GIT pull || exit 1
$DOCKER system prune

$DOCKER_COMPOSE -f compose.yml -f prod.compose.yml up --detach || exit 1
echo "done"
