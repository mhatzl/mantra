+++
product_id: "B"
+++

# `req-1`: First Requirement of Product B

- **Parents:** [{ id: "req-1", product_id: "A" }]

This requirement is the child to [req_link({ id: "req-1", product_id: "A" })].

# `req-2`: Second Requirement of Product B

- **Parents:** ["req-1", { id: "req-2", product_id: "A" }]

This requirement is the child to [req_link("req-1")] of product "B" and [req_link({ id: "req-2", product_id: "A" })].
