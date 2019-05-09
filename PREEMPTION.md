# Pre-emption notes

1. I need to change the type of Kubernetes batch job I'm using to get better termination semantics.
2. I need to replace `datums.pod_name text` and `datums.node_name text` with full-fledged `pods` and `nodes` tables and proper foreign key relationships.
3. I need to understand pre-emptible VM instances more thoroughly.


> For reference, we've observed from historical data that the average preemption rate varies between 5% and 15% per day per project, on a seven day average, occasionally spiking higher depending on time and zone. Keep in mind that this is an observation only: preemptible instances have no guarantees or SLAs for preemption rates or preemption distributions.

> Additionally, these preemptible VMs are given a Kubernetes label, cloud.google.com/gke-preemptible=true. Kubernetes labels can be used in the nodeSelector field for scheduling Pods to specific nodes.

Highly recommended:

```kubectl taint nodes [NODE_NAME] cloud.google.com/gke-preemptible="true":NoSchedule```

> 6. The processes in the Pod are sent the TERM signal.

Note that you *may not receive the TERM signal in time*. It's kind of an optional helpful thing.

- Background SIGTERM monitor in the worker? Some sort of "terminated" state?
- jobs and nodes tables
- Background thread in daemon. Polls nodes and pods.
