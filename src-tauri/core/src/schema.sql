PRAGMA foreign_keys=ON;
CREATE TABLE IF NOT EXISTS wallets(id TEXT PRIMARY KEY,name TEXT NOT NULL,kind TEXT NOT NULL,bank TEXT NOT NULL,digits TEXT NOT NULL DEFAULT '',opening INTEGER NOT NULL CHECK(typeof(opening)='integer' AND opening>=0),observed INTEGER);
CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY,email TEXT NOT NULL,provider TEXT NOT NULL,host TEXT NOT NULL,port INTEGER NOT NULL,wallet_id TEXT NOT NULL REFERENCES wallets(id),enabled INTEGER NOT NULL DEFAULT 1,auto_post INTEGER NOT NULL DEFAULT 0,folder TEXT,last_sync TEXT);
CREATE UNIQUE INDEX IF NOT EXISTS account_email ON accounts(lower(email));
CREATE TABLE IF NOT EXISTS mappings(account_id TEXT NOT NULL REFERENCES accounts(id),bank TEXT NOT NULL,wallet_id TEXT NOT NULL REFERENCES wallets(id),PRIMARY KEY(account_id,bank));
CREATE TABLE IF NOT EXISTS cursors(account_id TEXT NOT NULL REFERENCES accounts(id),mailbox TEXT NOT NULL,uid_validity INTEGER NOT NULL,last_uid INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(account_id,mailbox));
CREATE TABLE IF NOT EXISTS emails(id TEXT PRIMARY KEY,account_id TEXT NOT NULL REFERENCES accounts(id),mailbox TEXT NOT NULL,uid_validity INTEGER NOT NULL,uid INTEGER NOT NULL,message_id TEXT NOT NULL,gm_id TEXT,sender TEXT NOT NULL,subject TEXT NOT NULL,received_at TEXT NOT NULL,raw_snippet TEXT NOT NULL,status TEXT NOT NULL,reason TEXT NOT NULL,parsed TEXT,UNIQUE(account_id,mailbox,uid_validity,uid));
CREATE UNIQUE INDEX IF NOT EXISTS ingestion_message_id ON emails(account_id,message_id);
CREATE UNIQUE INDEX IF NOT EXISTS ingestion_gm_id ON emails(account_id,gm_id) WHERE gm_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS ledger(id TEXT PRIMARY KEY,wallet_id TEXT NOT NULL REFERENCES wallets(id),to_wallet_id TEXT REFERENCES wallets(id),kind TEXT NOT NULL CHECK(kind IN ('expense','income','transfer')),amount INTEGER NOT NULL CHECK(typeof(amount)='integer' AND amount>0 AND amount<=100000000000000),merchant TEXT NOT NULL,category TEXT NOT NULL,occurred_at TEXT NOT NULL,source TEXT NOT NULL CHECK(source IN ('Email','Manual','Notification')),reference TEXT,bank TEXT NOT NULL,email_id TEXT REFERENCES emails(id),note TEXT NOT NULL,deleted INTEGER NOT NULL DEFAULT 0, CHECK(kind!='transfer' OR (to_wallet_id IS NOT NULL AND to_wallet_id!=wallet_id)));
CREATE UNIQUE INDEX IF NOT EXISTS ledger_bank_reference ON ledger(wallet_id,bank,reference) WHERE reference IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ledger_email ON ledger(email_id) WHERE email_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ledger_occurred ON ledger(occurred_at);
CREATE TABLE IF NOT EXISTS preferences(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS audit_log(id INTEGER PRIMARY KEY AUTOINCREMENT,action TEXT NOT NULL,entity_id TEXT NOT NULL,detail TEXT NOT NULL,created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
CREATE TABLE IF NOT EXISTS sync_receipts(
 account_id TEXT NOT NULL REFERENCES accounts(id), mailbox TEXT NOT NULL,
 uid_validity INTEGER NOT NULL, uid INTEGER NOT NULL,
 PRIMARY KEY(account_id,mailbox,uid_validity,uid)
);
CREATE INDEX IF NOT EXISTS email_scan_idx ON emails(account_id,mailbox,uid_validity,uid);
CREATE TABLE IF NOT EXISTS notification_events(
 id TEXT PRIMARY KEY, package_name TEXT NOT NULL, title TEXT NOT NULL,
 body TEXT NOT NULL, timestamp INTEGER NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('posted','review','ignored')),
 reason TEXT NOT NULL, ledger_id TEXT UNIQUE REFERENCES ledger(id)
);
CREATE INDEX IF NOT EXISTS notification_status_idx ON notification_events(status,timestamp);
CREATE TABLE IF NOT EXISTS merchant_rules(
 merchant_key TEXT PRIMARY KEY,
 category TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
