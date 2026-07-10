---
product_id: "A"
---

# `req-1`: First Requirement of Product A

This requirement is the parent to [req_link({ id: "req-1", product_id: "B" })].

# `req-2`: Second Requirement of Product A

This requirement is the parent to [req_link({ id: "req-2", product_id: "B" })].

# `req-3`: Third Requirement of Product A

- **Parents:** [{ id: "req-2", product_id: "B" }, "req-2"]

This requirement is the child to [req_link("req-2")] of product "A" and [req_link({ id: "req-2", product_id: "B" })].

**Note:** Products should not have this kind of interdependency, but as long as no dependency cycles are formed,
mantra must be able to correctly resolve such hierarchies.
