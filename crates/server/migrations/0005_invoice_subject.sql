-- An optional one-line subject for an invoice, shown on its detail and PDF.
ALTER TABLE invoices
    ADD COLUMN subject text NOT NULL DEFAULT ''
        CONSTRAINT invoices_subject_check CHECK (char_length(subject) <= 200);
