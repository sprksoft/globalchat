DOCKER="sudo docker"

if [[ $* != *--restart* ]] ; then
  $DOCKER image rm --force smppserver-smppgc
fi
if [[ $* == *--nodbgenv* ]] ; then
$DOCKER compose -f compose.yml up || exit 1
else
$DOCKER compose -f compose.yml -f debugenv.compose.yml up || exit 1
fi
