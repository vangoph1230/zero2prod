-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES(
    'fee087a1-efe5-44bb-bd1c-4033883f1173',
    'admin',
    '$argon2d$v=19$m=15000,t=2,p=1$gtjtTTrDgqJAFC1iDUtE3Q$1iMYx/vRMhqmaygj9U274N1Mq4gboOhjxzHzNdYPhN8'
)