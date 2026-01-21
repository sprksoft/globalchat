set dotenv-load := true
set export := true

alias b := build
alias r := run
alias prep := sqlx-prepare

run: _is_docker_working
    $DOCKER_COMPOSE up --watch

build: _is_docker_working
    $DOCKER_COMPOSE build
    $DOCKER_COMPOSE up --watch

check: check_rust check_ts

check_rust: db-up
    $CARGO check

[working-directory("smppgc/client")]
check_ts:
    tsc

db-up: _is_docker_working
    $DOCKER_COMPOSE up --detach db

sqlx-prepare: _is_docker_working _is_rust_working db-up
    $CARGO sqlx prepare --workspace

    $DOCKER_COMPOSE up --watch

[working-directory('smppgc')]
sqlx-reset-db: _is_rust_working db-up
    $CARGO sqlx database reset

# checks that cargo and sqlx are installed
_is_rust_working:
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
_is_docker_working:
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
