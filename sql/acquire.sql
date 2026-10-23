INSERT INTO leases(name,owner,expires_at) VALUES(?,?,?)
ON CONFLICT(name) DO UPDATE SET owner=excluded.owner,expires_at=excluded.expires_at
WHERE leases.expires_at < ?
RETURNING owner
