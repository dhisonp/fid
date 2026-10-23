WITH versions AS (
  SELECT metadata, source_id, retrieved_at,
    ROW_NUMBER() OVER (PARTITION BY url ORDER BY retrieved_at DESC) AS version
  FROM documents WHERE retrieved_at<=? AND first_seen<=? AND retrieved_at>=?
), bounded AS (
  SELECT metadata, ROW_NUMBER() OVER (PARTITION BY source_id ORDER BY retrieved_at DESC) AS position
  FROM versions WHERE version=1
)
SELECT metadata FROM bounded WHERE position<=6 LIMIT 120
