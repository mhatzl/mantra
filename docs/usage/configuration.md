# Configuration

At the top level, *mantra* expects a list of `products`, with each product defining where data should be collected from. This allows to have multiple products in one *mantra* database, which is useful for example
in a monorepo.

For every product you must either set an `id`, or `name` and `base` to identify a product.
If no `id` is set it is generated as `<name>@<base>`.
Information such as `license` or `homepage` may be set, but is optional.
Product related information except the `id` may be set at top level outside the `products` list,
and then inherited by setting `$inherit` to remove duplication in case of multiple products.

To collect data for a product, you must define where data should be collected from.
*mantra* distinguishes between annotations, requirements, test-runs, and reviews.
Those are covered in more detail under [usage/collect](collect/README.md).

For reports, a path to custom templates may be set using `template_dir` under `reports`.
All files in the given directory that are in the same format as the targeted report output format
are then available and overwrite the [default templates](/mantra/src/cmd/report/templates/defaults) that are integrated into *mantra*.

Below is the configuration used for *mantra* itself:

```json5
{
  name: "mantra",
  license: "MIT",
  products: [{
    name: "$inherit",
    license: "$inherit",
    base: "main",
    requirements: [{
        path: "test-content/reqs/",
        source: "schema",
    }],
    annotations: [{
        path: "test-content/annotations/",
        source: "schema",
    },{
        path: "./",
        source: "content",
        pattern: "*.rs"
    }],
    test_runs: [{
        path: "target/nextest/default",
        source: {
            test: {
                format: "junit",
                pattern: "*junit.xml",
            },
            coverage: {
                format: "cobertura_loose",
                pattern: "*cobertura.xml",
            }
        }
    },{
        path: "test-content/test-runs/",
        source: "schema",
    }],
    reviews: [{
        path: "test-content/reviews/",
        source: "schema",
    }]
  }],
  reports: {
    template_dir: "test-content/custom-templates"
  }
}
```
