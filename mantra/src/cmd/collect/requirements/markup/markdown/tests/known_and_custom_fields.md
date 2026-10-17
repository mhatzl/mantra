# `req-1`: Title

- **Manual:** true
- mantra_unknown: "Some custom property value"

# `req-2`: Title

- Parents: ["req-1"]
- **Optional:** true
- **Custom-Map:** {
    key: "value",
  }
- Custom-List: [
    "entry-1",
    "entry-2",
  ]

# `req-3`: Title

- **Parents:** ["req-1"]
- some_flag: false

# `req-4`: Title

- **Children:** ["req-3"]
- **Parents:** [{ id: "req-1", product_id: "prod-1", optional: true}]

# `req-5`: Title

- **Parents:** [{ id: "req-1", aspect: "aspect-A" }, { id: "req-2", optional: true, aspect: "aspect-10" }]
