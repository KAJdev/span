-- WireGuard networking additions
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS wg_ip INET UNIQUE;
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS public_endpoint TEXT;