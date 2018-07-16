# Introduction

Falconeri is lightweight tool for running distributed batch jobs on a Kubernetes cluster. You can specify your processing code as a Docker image that reads files as input, and produces other files as output. This allows you to use virtually any programming language.

Falconeri will read files from cloud buckets, distribute them among multiple copies of your worker image, and collect the output into a another cloud bucket.

Falconeri is inspired by the open source [Pachyderm][], which offers a considerably richer set of tools for batch-processing on a Kubernetes cluster, plus a `git`-like file system for tracking multiple versions of data and recording the provenance.

[Pachyderm]: http://www.pachyderm.io/
