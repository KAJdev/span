-- Add cordoned status to nodes
ALTER TABLE nodes ADD COLUMN IF NOT EXISTS cordoned BOOLEAN DEFAULT FALSE;

-- Minimal table to track container deployments per node
CREATE TABLE IF NOT EXISTS container_deployments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    app_id UUID,
    node_id UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    container_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_container_deployments_node ON container_deployments(node_id);

-- Minimal service_endpoints table so node removal can clean up endpoints
CREATE TABLE IF NOT EXISTS service_endpoints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    node_id UUID NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    address TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_service_endpoints_node ON service_endpoints(node_id);
