# Accessing the database

Sometimes, you may need to know more about a job than you can access using the CLI. Falconeri stores its state in a PostgreSQL database, which you can access using the following commands.

**WARNING:** The database schema is subject to change without warning, and does not form part of the stable interface to Falconeri. Proceed at your own risk!

## `db url`

Print out a URL pointing to Falconeri's PostgreSQL server, as mapped by `falconeri proxy`:

```sh
falconeri db uri
```

## `db console`

Use `psql` to connect to Falconeri's PostgreSQL server, as mapped by `falconeri proxy`:

```sh
falconeri db console
```
