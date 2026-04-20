ALTER TABLE transactions ADD COLUMN realized_pnl INTEGER;
ALTER TABLE transactions ADD COLUMN created_at TEXT NOT NULL DEFAULT (datetime('now'));
