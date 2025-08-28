-- Add migration script here
--将整个迁移过程放入一个事务中，以确保原子化的成功或者失败
--注意：'Sqlx'并不会自动帮我们进行原子化处理
BEGIN;
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;