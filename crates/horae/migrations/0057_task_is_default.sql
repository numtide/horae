-- A common task: the new-project form adds it to the task list. Existing
-- projects and other ways of creating a project are unaffected.
ALTER TABLE tasks ADD COLUMN is_default boolean NOT NULL DEFAULT false;
