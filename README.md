# `falconeri`: A minimalist data pipeline app

To deploy the server component, make sure `kubectl` is pointed at the right server, and run:

```sh
# Generate secrets.
kubectl create secret generic falconeri \
    --from-literal=POSTGRES_PASSWORD="$(apg -MNCL -m32 -x32 -a 1 -n 1)"

# Deploy falconeri onto the cluster.
falconeri deploy

# Once the database is running, update our database schema.
falconeri migrate
```

To use Falconeri, run:

```sh
# Proxy the ports we need to localhost (including PostgreSQL).
# Run this in its own terminal.
falconeri proxy

# Run a job.
falconeri run pipeline-spec.json
```

You currently need to create the cluster job manually.
