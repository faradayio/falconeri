# `falconeri`: A minimalist data pipeline app

To deploy the server component, run:

```sh
# Generate secrets.
kubectl create secret generic falconeri \
    --from-literal=POSTGRES_PASSWORD="$(apg -MNCL -m32 -x32 -a 1 -n 1)"

# Deploy falconeri onto the cluster.
kubectl apply -f falconeri/src/manifest.yml

# Set DATABASE_URL locally using our cluster's secret.
export DATABASE_URL="postgres://postgres:$(kubectl get secret falconeri -o json | jq -r .data.POSTGRES_PASSWORD | base64 --decode)@localhost:5432/"

# Install `diesel` CLI tool.
cargo install -f diesel_cli

# Create database tables.
cd falconeri_common
diesel migration run
```
