-- Executed inside Database::open's IMMEDIATE transaction, before notification_events exists.
CREATE TABLE ledger_v3(
 id TEXT PRIMARY KEY, wallet_id TEXT NOT NULL REFERENCES wallets(id),
 to_wallet_id TEXT REFERENCES wallets(id),
 kind TEXT NOT NULL CHECK(kind IN ('expense','income','transfer')),
 amount INTEGER NOT NULL CHECK(typeof(amount)='integer' AND amount>0 AND amount<=100000000000000),
 merchant TEXT NOT NULL, category TEXT NOT NULL, occurred_at TEXT NOT NULL,
 source TEXT NOT NULL CHECK(source IN ('Email','Manual','Notification')),
 reference TEXT, bank TEXT NOT NULL, email_id TEXT REFERENCES emails(id),
 note TEXT NOT NULL, deleted INTEGER NOT NULL DEFAULT 0,
 CHECK(kind!='transfer' OR (to_wallet_id IS NOT NULL AND to_wallet_id!=wallet_id))
);
INSERT INTO ledger_v3(id,wallet_id,to_wallet_id,kind,amount,merchant,category,occurred_at,source,reference,bank,email_id,note,deleted)
 SELECT id,wallet_id,to_wallet_id,kind,amount,merchant,category,occurred_at,source,reference,bank,email_id,note,deleted FROM ledger;
DROP TABLE ledger;
ALTER TABLE ledger_v3 RENAME TO ledger;
-- Existing ledger indexes were dropped with the old table; schema.sql recreates them.
