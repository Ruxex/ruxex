DROP INDEX IF EXISTS idx_threads_provider;

ALTER TABLE threads DROP COLUMN model_provider;
