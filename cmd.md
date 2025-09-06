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


{"v":0,"name":"zero2prod",
"msg":"[ADDING A NEW SUBSCRIBER. - START]","level":30,"hostname":"DESKTOP-VEG4CGQ","pid":22500,"time":"2025-09-06T17:11:38.9266689Z","target":"zero2prod::routes::subscriptions","line":25,"file":"src\\routes\\subscriptions.rs","request_id":"66f61aa7-4c0e-4b40-a451-d8fc0c113034","subscriber_email":"thomas_mann#hostmail.con","subscriber_name":"Tom"}
{"v":0,"name":"zero2prod","msg":"[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE. - START]","level":30,"hostname":"DESKTOP-VEG4CGQ","pid":22500,"time":"2025-09-06T17:11:38.9271137Z","target":"zero2prod::routes::subscriptions","line":43,"file":"src\\routes\\subscriptions.rs","request_id":"66f61aa7-4c0e-4b40-a451-d8fc0c113034","subscriber_email":"thomas_mann#hostmail.con","subscriber_name":"Tom"}
{"v":0,"name":"zero2prod","msg":"[SAVING NEW SUBSCRIBER DETAILS IN THE DATABASE. - END]","level":30,"hostname":"DESKTOP-VEG4CGQ","pid":22500,"time":"2025-09-06T17:11:38.932182Z","target":"zero2prod::routes::subscriptions","line":43,"file":"src\\routes\\subscriptions.rs","request_id":"66f61aa7-4c0e-4b40-a451-d8fc0c113034","subscriber_email":"thomas_mann#hostmail.con","elapsed_milliseconds":4,"subscriber_name":"Tom"}
{"v":0,"name":"zero2prod","msg":"[ADDING A NEW SUBSCRIBER. - EVENT] Failed to execute query: Database(PgDatabaseError { severity: Error, code: \"23505\", message: \"重复键违反唯一约束\\\"subscriptions_email_key\\\"\", detail: Some(\"键值\\\"(email)=(thomas_mann#hostmail.con)\\\" 已经存在\"), hint: None, position: None, where: None, schema: Some(\"public\"), table: Some(\"subscriptions\"), column: None, data_type: None, constraint: Some(\"subscriptions_email_key\"), file: Some(\"nbtinsert.c\"), line: Some(673), routine: Some(\"_bt_check_unique\") })","level":50,"hostname":"DESKTOP-VEG4CGQ","pid":22500,"time":"2025-09-06T17:11:38.932741Z","target":"zero2prod::routes::subscriptions","line":70,"file":"src\\routes\\subscriptions.rs","request_id":"66f61aa7-4c0e-4b40-a451-d8fc0c113034","subscriber_email":"thomas_mann#hostmail.con","subscriber_name":"Tom"}
{"v":0,"name":"zero2prod",
"msg":"[ADDING A NEW SUBSCRIBER. - END]",
"request_id":"66f61aa7-4c0e-4b40-a451-d8fc0c113034",
"subscriber_email":"thomas_mann#hostmail.con",
"elapsed_milliseconds":6,
"subscriber_name":"Tom"
}
