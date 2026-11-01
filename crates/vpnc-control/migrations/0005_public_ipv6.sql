-- Public IPv6 endpoint host + lock auto-detection when admin edits.
ALTER TABLE nodes ADD COLUMN public_ipv6 TEXT;
ALTER TABLE nodes ADD COLUMN public_endpoint_manual INTEGER NOT NULL DEFAULT 0;
