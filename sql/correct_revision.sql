INSERT INTO revisions(edition_date,revision,created_at,reason,payload)
SELECT ?,?,?,?,?
WHERE EXISTS(SELECT 1 FROM editions WHERE edition_date=? AND current_revision=?)
