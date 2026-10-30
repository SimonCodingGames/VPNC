-- Soft-disable flag for users (admin can deactivate without hard delete)
ALTER TABLE users ADD COLUMN disabled_at TEXT;
