CREATE USER 'kubellm'@'%' identified by 'modelme';
CREATE DATABASE IF NOT EXISTS kubellm;
GRANT ALL PRIVILEGES ON kubellm.* TO 'kubellm'@'%';
FLUSH PRIVILEGES;

CREATE TABLE IF NOT EXISTS kubellm.prompts (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    prompt TEXT NOT NULL,
    response MEDIUMTEXT NOT NULL,
    model VARCHAR(255) NOT NULL,
    provider VARCHAR(255) NOT NULL,
    created_at DATETIME NOT NULL
);