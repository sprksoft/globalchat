set dotenv-load
set export

alias b := build
alias r := run
alias prep := sqlx-prepare

run: _docker-check
  $DOCKER_COMPOSE up --watch

build: _docker-check
  $DOCKER_COMPOSE build
  $DOCKER_COMPOSE up --watch

check: db-up
  $CARGO check

db-up: _docker-check
  $DOCKER_COMPOSE up --detach db

sqlx-prepare: _docker-check _rust-check db-up
  $CARGO sqlx prepare --workspace

  $DOCKER_COMPOSE up --watch

[working-directory: 'smppgc']
sqlx-reset-db: _rust-check db-up
  $CARGO sqlx database reset



# checks that cargo and sqlx are installed
_rust-check:
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
_docker-check:
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

