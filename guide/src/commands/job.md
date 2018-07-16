# Running jobs

## `job run`

To run a job using `falconeri`, use the `job run` subcommand:

```sh
falconeri job run $PIPELINE_SPEC_JSON_PATH
```

The `$PIPELINE_SPEC_JSON_PATH` should point a file in [pipeline spec JSON](./specification.md) format. This will create all the necessary records for a job in the database, and start a job on the Kubernetes cluster. It will also print out the ID of the new job.

## `job list`

To list all known jobs, and their current state, run:

```sh
falconeri job list
```

## `job describe`

To see a summary of the current state of a job, including datums currently being processed, see:

```sh
falconeri job describe $JOB_NAME
```

## `datum describe $DATUM_ID`

To describe an individual datum in a job, you can run:

```sh
falconeri datum describe $DATUM_ID
```

## `job retry`

If a job has failed due to an intermittent error, you can re-run just the failed datums using `job retry`:

```sh
falconeri job retry $JOB_NAME
```

Note that this will use the original pipeline specification JSON, and that it will create a new job.

**KLUDGE:** If you need to edit the pipeline spec JSON before retrying, you might be able to do so using `falconeri db console` to change the `jobs.pipeline_spec` column. Note that this is not officially supported.
