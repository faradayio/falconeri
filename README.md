# `falconeri`: Run batch data-processing jobs on Kubernetes

Falconeri runs on a pre-existing Kubernetes cluster, and it allows you to use Docker images to transform large data files stored in cloud buckets.

For detailed instructions, see the [Falconeri guide][guide].

Setup is simple:

```sh
falconeri deploy
falconeri proxy
falconeri migrate
```

Running is similarly simple:

```sh
falconeri job run my-job.json
```

[guide]: https://github.com/faradayio/falconeri/blob/master/guide/src/SUMMARY.md
