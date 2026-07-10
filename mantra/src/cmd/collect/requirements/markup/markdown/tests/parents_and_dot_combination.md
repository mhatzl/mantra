# `req-1`: First Requirement Title

Description for the first requirement.

## `req-1.sub-1`: First Sub-Requirement of the First Requirement

- **Parents:** ["req-1"]

Description of the first sub-requirement of [req_link("req-1")].
This description is not part of the description for [req_link("req-1")].

**Note:** Setting `req-1` explicitly in the parent field has no effect, since the dot-notation already marks `req-1` as parent.

## `req-1.sub-2`: Second Sub-Requirement of the First Requirement

Description of the second sub-requirement of [req_link("req-1")].
This description is not part of the description for [req_link("req-1")].

# `req-2`: Second Requirement Title

Description for the second requirement.

## `req-2.sub-1`: First Sub-Requirement of the Second Requirement

- **Parents:** ["req-1", "req-2"]

Description of the first sub-requirement of [req_link("req-2")].
This description is not part of the description for [req_link("req-2")].

The requirement is also a child of [req_link("req-1")].
Explicitly setting `req-2` in the parents field is optional.

# `req-3`: Third Requirement Title

- **Parents:** ["req-1"]

Description for the third requirement.
The requirement is also a child of [req_link("req-1")].

## `req-3.sub-1`: First Sub-Requirement of the Third Requirement

Description of the first sub-requirement of [req_link("req-3")].
This description is not part of the description for [req_link("req-3")].

### `req-3.sub-1.sub-sub-1`: First Sub-Sub-Requirement of the Third Requirement

Description of the first sub-sub-requirement of [req_link("req-3.sub-1")].
This description is not part of the description for [req_link("req-3.sub-1")].

### `req-3.sub-1.sub-sub-2`: Second Sub-Sub-Requirement of the Third Requirement

Description of the second sub-sub-requirement of [req_link("req-3.sub-1")].
This description is not part of the description for [req_link("req-3.sub-1")].

### `req-3.sub-1.sub-sub-3`: Third Sub-Sub-Requirement of the Third Requirement

- **Parents:** ["req-3.sub-1.sub-sub-1", "req-3.sub-1.sub-sub-2"]

Description of the third sub-sub-requirement of [req_link("req-3.sub-1")].
This description is not part of the description for [req_link("req-3.sub-1")].

The requirement is also a child of [req_link("req-3.sub-1.sub-sub-1")] and [req_link("req-3.sub-1.sub-sub-2")].
