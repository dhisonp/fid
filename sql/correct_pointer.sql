UPDATE editions SET current_revision=?
WHERE edition_date=? AND current_revision=?
AND EXISTS(SELECT 1 FROM revisions WHERE edition_date=? AND revision=?)
