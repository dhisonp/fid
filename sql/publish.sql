INSERT INTO editions(edition_date,cutoff,generation_started_at,published_at)
SELECT ?,?,?,unixepoch()
WHERE EXISTS(SELECT 1 FROM leases WHERE name='generation' AND owner=? AND expires_at>unixepoch())
AND unixepoch()<?
