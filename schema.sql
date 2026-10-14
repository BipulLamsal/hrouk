CREATE TABLE IF NOT EXISTS users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    endpoint_id VARCHAR(128) NOT NULL UNIQUE,
    display_name VARCHAR(255) NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_users_email (email),
    INDEX idx_users_endpoint (endpoint_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS presence_log (
    id INT AUTO_INCREMENT PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    endpoint_id VARCHAR(128) NOT NULL,
    event ENUM('login', 'logout') NOT NULL,
    authorized TINYINT(1) NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_presence_email (email),
    INDEX idx_presence_time (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
