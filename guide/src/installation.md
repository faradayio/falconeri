# Installation

To install `falconeri`, you'll need a working Kubernetes cluster and a basic knowledge of Kubernetes. Ideally, your Kubernetes cluster should support cluster autoscaling, allowing it to automatically add and remove servers as needed.

We've had good luck with the following:

- Google Kubernetes Engine, running version 1.10.5-gke.0 with permissions `--scopes=gke-default,storage-rw` and SSD boot disks.
- O'Reilly's _Kubernetes: Up & Running_, which introduces nearly all the Kubernetes features used by Falconeri.

## Required software

If you're running Kubernetes on Google's cloud, you will need:

- `gsutil` for accessing Google Cloud Storage.
- `gcloud` for working with your cluster.

For other clouds, you will need to check your vendor's documentation.

For all setups, you will also need:

- `kubectl`, compatible with your version of Kubernetes.
- `falconeri`, which you should be able to find on the [releases page][releases].

[releases]: https://github.com/faradayio/falconeri/

## Setting up a cluster autoscaling pool

We've had good luck splitting our cluster into two separate parts:

1. Three master nodes. These run the Kubernetes cluster infrastructure, plus the Falconeri back end. These are always running. For a really small installation, it might be possible to get by with a single master node.
2. A cluster autoscaling pool for the worker nodes. This will grow and shrink automatically as needed to accomodate the batch jobs run by Falconeri.

If you're running on Google, and you have a cluster named `falconeri` in `$CLUSTER_ZONE`, you can use a command like the following to add a worker node pool:

```sh
# Authenticate with your Falconeri cluster.
gcloud container clusters get-credentials falconeri \
    --zone $CLUSTER_ZONE --project $CLUSTER_PROJECT

# Set your Falconeri cluster as the default for kubectl commands.
kubectl config set-cluster falconeri

# Create the worker node pool (adjust parameters as needed).
gcloud container node-pools create falconeri-workers \
    --cluster=falconeri --disk-size=1000 --enable-autorepair \
    --machine-type=n1-standard-8 --node-version=1.10.5-gke.0 \
    --node-labels=node_type=falconeri_worker --disk-type pd-ssd \
    --num-nodes=0 --enable-autoscaling --min-nodes=0 --max-nodes=25 \
    --zone=$CLUSTER_ZONE --scopes=gke-default,storage-rw
```

Strictly speaking, this is optional. But in practice, autoscaling is extremely convenient, and isolating the workers to a separate node pool will reduce the risk of a runaway worker process "evicting" critical Kubernetes infrastructure.

## Deploying Falconeri

Once you are authenticated with your cluster and you have added your autoscaling pool, you can install Falconeri as follows:

```sh
# Install the software.
falconeri deploy

# Wait for the falconeri-postgres pod to become ready.
kubectl get pods

# (In a separate terminal.) Set up a proxy connection to Falconeri.
falconeri proxy

# Update your database to the latest schema.
falconeri migrate
```
