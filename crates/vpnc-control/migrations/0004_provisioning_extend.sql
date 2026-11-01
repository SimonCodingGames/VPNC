-- Extra fields for dashboard SSH provisioning jobs.
ALTER TABLE provisioning_jobs ADD COLUMN node_name TEXT;
ALTER TABLE provisioning_jobs ADD COLUMN tunnel_cidr_v4 TEXT;
ALTER TABLE provisioning_jobs ADD COLUMN tunnel_cidr_v6 TEXT;
ALTER TABLE provisioning_jobs ADD COLUMN observed_host_key TEXT;
