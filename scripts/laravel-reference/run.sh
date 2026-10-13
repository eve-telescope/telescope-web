#!/usr/bin/env bash
# Runs the Laravel reference (the `main` branch) on http://127.0.0.1:8318,
# with its Reverb server on ws://127.0.0.1:8319, for recording golden
# fixtures (record.py for HTTP, pusher.mjs for the websocket frames).
#
#   scripts/laravel-reference/run.sh setup         export main, install, build assets
#   scripts/laravel-reference/run.sh start [ping]  fresh database, serve; ping: Reverb's
#                                                  ping interval in seconds (default 60)
#   scripts/laravel-reference/run.sh artisan ...   run artisan in the reference container
#   scripts/laravel-reference/run.sh php '...'     run PHP code with the app booted (tinker)
#   scripts/laravel-reference/run.sh sql "..."     run SQL against the reference database
#   scripts/laravel-reference/run.sh stop
#
# The checkout lives in $LARAVEL_REF_DIR (default /tmp/telescope-laravel-ref),
# never inside this repository. MySQL 8 like production (case-insensitive
# collation), the `log` mailer, no other services. Containers are named
# telescope-laravel-*, on the telescope-laravel network.
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
dir=${LARAVEL_REF_DIR:-/tmp/telescope-laravel-ref}
image=telescope-laravel-ref
network=telescope-laravel
port=8318
reverb_port=8319

run() {
  docker run --rm -u "$(id -u):$(id -g)" -e HOME=/tmp -e COMPOSER_HOME=/tmp/composer \
    --network "$network" -v "$dir/app:/app" "$@"
}

mysql_up() {
  docker network inspect "$network" >/dev/null 2>&1 || docker network create "$network" >/dev/null
  if ! docker inspect telescope-laravel-mysql >/dev/null 2>&1; then
    docker run -d --name telescope-laravel-mysql --network "$network" \
      -e MYSQL_ROOT_PASSWORD=telescope -e MYSQL_DATABASE=laravel \
      mysql:8.4 --character-set-server=utf8mb4 --collation-server=utf8mb4_unicode_ci >/dev/null
  fi
  until docker exec telescope-laravel-mysql mysql -uroot -ptelescope -e 'select 1' >/dev/null 2>&1; do sleep 1; done
}

case "${1:-}" in
  setup)
    docker build -t "$image" "$repo/scripts/laravel-reference"
    rm -rf "$dir" && mkdir -p "$dir/app"
    git -C "$repo" archive main | tar -x -C "$dir/app"
    cp "$repo/scripts/laravel-reference/env" "$dir/app/.env"
    mysql_up
    run --name telescope-laravel-setup "$image" sh -c '
      composer install --no-interaction --no-progress
      npm ci --no-audit --no-fund
      php artisan wayfinder:generate --with-form || true
      npx vite build'
    ;;
  start)
    ping=${2:-60}
    mysql_up
    rm -f "$dir/app/storage/logs/laravel.log"
    docker exec telescope-laravel-mysql mysql -uroot -ptelescope \
      -e 'drop database if exists laravel; create database laravel' 2>/dev/null
    run --name telescope-laravel-migrate "$image" php artisan migrate --force >/dev/null
    docker rm -f telescope-laravel-ref >/dev/null 2>&1 || true
    docker run -d --name telescope-laravel-ref -u "$(id -u):$(id -g)" -e HOME=/tmp \
      -e PHP_CLI_SERVER_WORKERS=4 -e REVERB_APP_PING_INTERVAL="$ping" \
      --network "$network" -v "$dir/app:/app" \
      -p "127.0.0.1:$port:$port" -p "127.0.0.1:$reverb_port:8080" "$image" \
      sh -c "php artisan reverb:start --host=0.0.0.0 --port=8080 & \
             exec php artisan serve --host=0.0.0.0 --port=$port --no-reload" >/dev/null
    until curl -fs "http://127.0.0.1:$port/up" >/dev/null; do sleep 0.5; done
    echo "Laravel reference on http://127.0.0.1:$port, Reverb on ws://127.0.0.1:$reverb_port"
    ;;
  artisan)
    shift
    docker exec telescope-laravel-ref php artisan "$@"
    ;;
  php)
    shift
    docker exec -i telescope-laravel-ref php artisan tinker --execute="$*"
    ;;
  sql)
    shift
    docker exec -i telescope-laravel-mysql mysql -uroot -ptelescope --default-character-set=utf8mb4 -N -B laravel -e "$*" 2>/dev/null
    ;;
  stop)
    docker rm -f telescope-laravel-ref telescope-laravel-mysql >/dev/null 2>&1 || true
    docker network rm "$network" >/dev/null 2>&1 || true
    ;;
  *)
    sed -n '2,16p' "$0"
    exit 1
    ;;
esac
