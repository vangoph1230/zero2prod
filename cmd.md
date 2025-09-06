sqlx-cli工具：
only for Postgres:
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
----------------------------------------------------------------------------------------

.env文件的意义：
# sqlx在编译时与Postgres进行交互，已检查查询是否合法，
# 但依赖DATABASE_URL环境变量确认数据库的位置。
# sqlx将从.env文件中读取DATABASE_URL,省去了每次都要重新导出环境变量的麻烦。
# .env文件 仅与 开发过程、构建和测试步骤相关。
-----------------------------------------------------------------------------------------

curl http://127.0.0.1:8000/health_check -v

curl -i -X POST -d 'email=thomas_mann#hostmail.con&name=Tom' http://127.0.0.1:8000/subscriptions

-----------------------------------------------------------------------------------------
