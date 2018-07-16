# Job specification

Here is a sample job specification:

```json
{{#include ../../falconeri/src/example_pipeline_spec.json}}
```

Some notes:

- `parallelism_spec` only accepts `constant`, not `coefficient`. We don't scale the job to fit the cluster; we scale the cluster to fit the job.
- `resource_requests` is mandatory.
- The `resource_requests.memory` value is used as both a request and as a hard limit. This is because we've seen too many problems caused by worker nodes that consume unexpectedly large amounts of RAM, forcing other workers (or cluster infrastructure) to be evicted from the node.
- `node_selector` is optional. When present, it allows you to limit which nodes will be used for workers. This also integrates with Kubernetes cluster autoscaling. The autoscaler will look for a node pool with matching tags, and create as many nodes as required to satisfy the `resource_requests`.
- For now, `input.atom` is the only supported input type.
- `egress.URI` is mandatory.
