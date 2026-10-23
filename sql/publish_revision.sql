INSERT INTO revisions(edition_date,revision,created_at,payload)
SELECT ?1,1,published_at,json_set(?2,'$.published_at',published_at)
FROM editions WHERE edition_date=?1
