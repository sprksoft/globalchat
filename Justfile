set dotenv-load
set export

alias b := build-run
alias r := run
alias sqlx := sqlx-build-run

# rebuild and run
build-run: build run
# run sqlx prepare, rebuild and run
sqlx-build-run: sqlx-prepare build run

run: check
  $DOCKER_COMPOSE -f compose.yml up --watch

build: check
  $DOCKER_COMPOSE -f compose.yml build

sqlx-prepare: check sqlx-check
  @echo "pulling db up..."
  $DOCKER_COMPOSE -f db.compose.yml up --detach
  @echo "sqlx prepare..."
  $CARGO sqlx prepare --workspace

# checks that cargo and sqlx are installed
sqlx-check:
  #!/usr/bin/env bash
  if ! $CARGO version >> /dev/null ; then
    echo "Cargo not found"
    exit 1
  fi
  if ! type sqlx >> /dev/null ; then
    echo "sqlx not installed. Use 'cargo install sqlx' to install"
    exit 1
  fi

# checks that docker and docker-compose are working
check:
  #!/usr/bin/env bash
  if ! $DOCKER version >> /dev/null ; then
  echo "failed to run $DOCKER version. posible reasons:"
  echo "- docker is not installed"
  echo "- sudo canceled"
  echo "- docker has updated and no system restart has happened"
  exit 1
  fi

  if ! $DOCKER_COMPOSE version >> /dev/null ; then
  echo "failed to run $DOCKER_COMPOSE version. posible reasons:"
  echo "- docker is not installed"
  echo "- sudo canceled"
  echo "- docker has updated and no system restart has happened"
  exit 1
  fi

