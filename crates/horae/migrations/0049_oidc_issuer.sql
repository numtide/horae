-- A subject is stable only within its issuer. Existing bindings keep their
-- subject with no issuer: sign-in never guesses one, so they stay unusable until
-- an operator records it with `horae user link-oidc`.
ALTER TABLE users
  ADD COLUMN oidc_issuer text,
  ADD CONSTRAINT users_oidc_issuer_requires_subject
    CHECK (oidc_issuer IS NULL OR oidc_subject IS NOT NULL);
