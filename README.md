# `falconeri`: A minimalist data pipeline app

To deploy the server component, make sure `kubectl` is pointed at the right server, and run:

```sh
# Deploy falconeri onto the cluster.
falconeri deploy

# Proxy the ports we need to localhost (including PostgreSQL).
# Run this in its own terminal.
falconeri proxy

# Once the database is running, update our database schema.
falconeri migrate
```

To use Falconeri, run:

```sh
# We need this running in another terminal whenever we connect.
falconeri proxy

# Run a job.
falconeri run pipeline-spec.json
```

## Autoscaling notes

Autoscaling the cluster, assuming your cluster is named `falconeri`:

```sh
gcloud container node-pools create falconeri-workers --cluster=falconeri \
    --disk-size=1000 --enable-autorepair --enable-autoupgrade \
    --machine-type=n1-standard-8 --node-version=1.10.5-gke.0 \
    --node-labels=fdy.io/node_type=falconeri_worker --disk-type pd-ssd \
    --num-nodes=0 --enable-autoscaling --min-nodes=0 --max-nodes=25 \
    --zone=us-east1-b --scopes=gke-default,bigquery,storage-rw
```
