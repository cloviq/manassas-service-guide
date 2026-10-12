-- Additive migration: original seeds and tickets remain untouched.
ALTER TABLE services DROP CONSTRAINT IF EXISTS services_category_check;
ALTER TABLE services ADD COLUMN record_type text NOT NULL DEFAULT 'service' CHECK (record_type IN ('service','place','route','outage','infrastructure'));
ALTER TABLE services ADD COLUMN source_key text NOT NULL DEFAULT 'seed';
ALTER TABLE services ADD COLUMN source_record_id text NOT NULL DEFAULT '';
ALTER TABLE services ADD COLUMN attributes jsonb NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE services ADD COLUMN geometry jsonb;
CREATE INDEX services_type_category ON services(record_type,category);
CREATE INDEX services_provenance ON services(source_key,source_record_id);
