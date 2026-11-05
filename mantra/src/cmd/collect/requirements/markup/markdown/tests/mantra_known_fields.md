# `req-1`: Title

- **Manual:** true

# `req-2`: Title

- Parents: ["req-1"]
- **Optional:** true

Some description for the requirement.

# `req-3`: Title

- **Parents:** ["req-1"]
- **Manual Verification:** true
- **Deprecated:** true
- **Exclude:** true
- **Replaces:** ["req-2"]

# `req-4`: Title

- **Parents:** [{ id: "req-3", optional: true, aspect: "some-aspect" }]
- **Kind:** "riks"
- **Draft:** true
