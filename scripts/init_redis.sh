# scripts/init_redis.sh
#!/usr/bin/env bash
set -x
set -eo pipefail

#如果Redis容器正在运行，则打印指令，杀死进程后退出
RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
    echo >&2 "there is a redis container already running, kill it with"
    echo >&2 " docker kill ${RUNNING_CONTAINER}"
    exit 1
fi

#通过Docker启动Redis
docker run \
    -p "6379:6379" \
    -d \
    --name "redis_$(date '+%s')" \
    redis:6

echo >&2 "Redis is ready to go!"