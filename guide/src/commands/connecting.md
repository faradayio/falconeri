# Connecting

Before using Falconeri, you'll need to authenticate with your Kubernetes cluster using `kubectl`, and then set up a proxy connection.

## `proxy`

Before doing anything else, you will need to connect to Flaconeri using the `proxy` command:

```sh
falconeri proxy
```

This currently maps Falconeri's PostgreSQL server to `localhost:5432`. In the future, it may also map a second port for access to a Falconeri server.

You will need to make sure that this is running every time you use Falconeri.
