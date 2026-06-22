# *mantra* JSON Schema Generation

This crate is used to generate *mantra*'s JSON schemas that are defined in the `mantra-schema` crate.
The schemas are grouped in the ones used to *collect* information and the once used to create *report*s.

Adhering to the schemas used to *collect* information allows to use third-party tools to collect information.
The *report* schemas may be used to represent *mantra*'s output in a custom format, or to run simple analysis on the output data.
