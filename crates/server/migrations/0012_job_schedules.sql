-- Stored schedules (spec 027): each enqueues a job kind every N seconds, or
-- daily or weekly at a local time in UTC, the organization's or a person's zone.

CREATE TABLE horae_job_schedules (
    id uuid PRIMARY KEY,
    org_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    name text NOT NULL,
    kind text NOT NULL,
    payload jsonb NOT NULL,
    cadence text NOT NULL CHECK (cadence IN ('every', 'daily', 'weekly')),
    every_secs integer CHECK (every_secs BETWEEN 60 AND 86400),
    local_time time,
    weekday smallint CHECK (weekday BETWEEN 1 AND 7),
    zone_source text NOT NULL CHECK (zone_source IN ('utc', 'organization', 'person')),
    person_id uuid,
    concurrency_key text,
    -- Passed to each job, as an enqueuer would from the kind's policy.
    priority smallint NOT NULL DEFAULT 0,
    max_attempts integer NOT NULL CHECK (max_attempts >= 1),
    enabled boolean NOT NULL DEFAULT true,
    -- Null until the scheduler first plans the schedule (a daily or weekly
    -- one needs its zone, which only the scheduler resolves).
    next_due_at timestamptz,
    last_fired_at timestamptz,
    UNIQUE (org_id, name),
    FOREIGN KEY (person_id, org_id) REFERENCES users (id, org_id) ON DELETE CASCADE,
    CHECK (CASE cadence
        WHEN 'every' THEN every_secs IS NOT NULL AND local_time IS NULL AND weekday IS NULL
        WHEN 'daily' THEN every_secs IS NULL AND local_time IS NOT NULL AND weekday IS NULL
        ELSE every_secs IS NULL AND local_time IS NOT NULL AND weekday IS NOT NULL
    END),
    CHECK ((zone_source = 'person') = (person_id IS NOT NULL))
);

CREATE INDEX horae_job_schedules_due_idx ON horae_job_schedules (next_due_at) WHERE enabled;
