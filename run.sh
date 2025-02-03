DOCKER="sudo docker"

if [[ "$1" == "--nodebugdb" ]] ; then
$DOCKER compose up -f compose.yml || exit 1
else
$DOCKER compose up -f debug.compose.yml || exit 1
fi

