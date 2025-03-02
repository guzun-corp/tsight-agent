DROP DATABASE IF EXISTS test_db;

CREATE DATABASE IF NOT EXISTS test_db;

CREATE TABLE IF NOT EXISTS test_db.orders
(
    id UUID DEFAULT generateUUIDv4(),
    user_id UUID,
    notification_recipient_email String,
    order_name String,
    order_cost Decimal(15,2),
    created_at DateTime DEFAULT now(),
    updated_at DateTime DEFAULT now(),
    status String,
    is_deleted Bool DEFAULT false
) ENGINE = MergeTree()
ORDER BY (created_at, id);

-- Insert some test data
INSERT INTO test_db.orders 
(user_id, notification_recipient_email, order_name, order_cost, status, created_at)
VALUES
(generateUUIDv4(), 'user0@example.com', 'First Order', 100.50, 'new', '2025-01-30 23:59:00'),
(generateUUIDv4(), 'user1@example.com', 'First Order', 100.50, 'new', '2025-01-30 23:58:00'),
(generateUUIDv4(), 'user2@example.com', 'Second Order', 200.75, 'processing', '2025-01-30 23:57:00'),
(generateUUIDv4(), 'user3@example.com', 'Third Order', 150.25, 'completed', '2025-01-30 23:56:00'),
(generateUUIDv4(), 'user3@example.com', 'Third Order', 150.50, 'cancelled', '2025-01-30 23:55:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:54:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:54:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:54:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:53:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:52:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:51:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, '4222 2222 2222 2', '2025-01-30 23:51:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'user4@example.com', '2025-01-30 23:51:00'),
(generateUUIDv4(), 'user4@example.com', 'Fourth Order', 300.00, 'cancelled', '2025-01-30 23:45:00');


-- for testing filtering tables
CREATE TABLE IF NOT EXISTS test_db.card_numbers
(
    id UUID DEFAULT generateUUIDv4(),
    card_number String,
    is_deleted Bool DEFAULT false
) ENGINE = MergeTree()
ORDER BY (id);


INSERT INTO test_db.card_numbers
(card_number)
VALUES
('4222222222222'),
('42222 222 22222'),
('5123450000000008'),
('5123 4500 0000 0008'),
('3530111333300000'); -- we will not exclude jcb cards in tests


CREATE TABLE IF NOT EXISTS test_db.some_secret_db
(
    id UUID DEFAULT generateUUIDv4(),
    master_password String
) ENGINE = MergeTree()
ORDER BY (id);


INSERT INTO test_db.some_secret_db
(master_password)
VALUES
('qwerty123456'),
('1985secretPasswordApril03');

CREATE TABLE IF NOT EXISTS test_db._my_lovely_tmp_table
(
    id UUID DEFAULT generateUUIDv4(),
    country String,
    currency String
) ENGINE = MergeTree()
ORDER BY (id);


INSERT INTO test_db._my_lovely_tmp_table
(country, currency)
VALUES
('US', 'USD'),
('GR', 'EUR'),
('CY', 'EUR'),
('UK', 'GBP'), 
('SA', 'SAR'); 


CREATE DATABASE IF NOT EXISTS _test_db2;

CREATE TABLE IF NOT EXISTS _test_db2.table1
(
    id UUID DEFAULT generateUUIDv4(),
    data String
) ENGINE = MergeTree()
ORDER BY (id);

INSERT INTO _test_db2.table1
(data)
VALUES
('data');

CREATE TABLE IF NOT EXISTS _test_db2.table2
(
    id UUID DEFAULT generateUUIDv4(),
    data String
) ENGINE = MergeTree()
ORDER BY (id);

INSERT INTO _test_db2.table2
(data)
VALUES
('data2');
