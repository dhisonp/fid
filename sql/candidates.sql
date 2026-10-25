WITH versions AS (
  SELECT metadata, source_id, retrieved_at,
    ROW_NUMBER() OVER (PARTITION BY url ORDER BY retrieved_at DESC) AS version
  FROM documents WHERE retrieved_at<=? AND first_seen<=? AND retrieved_at>=?
), bounded AS (
  SELECT metadata, source_id, ROW_NUMBER() OVER (PARTITION BY source_id ORDER BY retrieved_at DESC) AS position
  FROM versions WHERE version=1
)
SELECT metadata FROM bounded
WHERE position <= CASE source_id WHEN 'hacker-news' THEN 10 WHEN 'x-search' THEN 8 ELSE 6 END
ORDER BY CASE WHEN source_id IN ('hacker-news', 'x-search') THEN 0 ELSE 1 END, position, source_id
LIMIT 120
