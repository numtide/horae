-- Name equivalence must not depend on the cluster's LC_CTYPE. In particular,
-- the C locale lowercases only ASCII. ICU's root locale covers Unicode while
-- deterministic comparison keeps accents and normalization forms distinct.
CREATE COLLATION permission_template_names (
    provider = icu,
    locale = 'und',
    deterministic = true
);

-- sqlx runs this migration transactionally: pre-existing equivalent names make
-- the index replacement fail without dropping the old index or changing rows.
DROP INDEX permission_templates_org_name_key;
CREATE UNIQUE INDEX permission_templates_org_name_key
    ON permission_templates (org_id, lower(name COLLATE permission_template_names));
