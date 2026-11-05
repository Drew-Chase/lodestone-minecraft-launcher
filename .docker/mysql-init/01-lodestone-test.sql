CREATE DATABASE IF NOT EXISTS lodestone_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
GRANT ALL PRIVILEGES ON lodestone.* TO 'lodestone'@'%';
GRANT ALL PRIVILEGES ON lodestone_test.* TO 'lodestone'@'%';
FLUSH PRIVILEGES;
