# PGAF: Parallel Geospatial Analysis Framework

Work-In-Progress lightweight tool for mass-parallel geospatial analysis.

The `pgaf` is an early-stage WIP command-line tool for running mass-parallel point-based data analysis. The premise is taking individual points (`id`, `lat`, `lon`) of a source GIS file (in `pgaf`-terms, a "domain") and streaming them through a series of transformers (in `pgaf`-terms, a "pipeline"), then collecting this stream into a meaningful output (in `pgaf`-terms, a "sink").

This is still a very early prototype and definitely not ready for any real-life workloads. However, it's constantly being worked on and I expect to have a somewhat more usable tool soon.

## Basic concepts and building blocks

Below, lies a **conceptual** representation of a workload spec file. Please note that due to the very early-stage, this example **does not work**, but this is more or less what I'm aiming for.

```yaml
domain:
  type: std::raster
  input_file: ./data/elevation.tif

pipeline:
  - name: add_location_info
    type: std::map
    args:
      where_are_we: "We are on, ${lat},${lon}, and our execution unit ID is ${id}."

  - name: calculate_adjusted_elevation
    type: std::map
    args:
        adjusted_elevation: "$={sum(a: base_elevation, b:  2.5)}"

  - name: filter_invalid_points
    type: std::filter
    args: "$={adjusted_elevation > 0}"

sink:
  - name: export_geotiff
    type: std::geotiff
    args:
      output_file: ./output/adjusted_elevation.tif

  - name: export_metrics_csv
    type: std::csv
    args:
      output_file: ./output/metrics.csv
      columns: ["id", "lat", "lon", "adjusted_elevation"]
```

### Domain

Domain is what your data source is called. Every workload has exactly one domain, where it will read a stream of points from (assigning each point an `id`, `lat` and `lon`; points are called Execution Units). For defining the domain in your workload spec file, you must first find the appropriated Domain Driver for your input file (e.g. raster readers, vector readers, synthetic domain generators).

#### Context

Execution Units are then transformed into a Context -- an Execution Unit + a dictionary of data that may or may not be mutated later. Then, the contexts are streamed into a [Pipeline](#pipeline).

### Pipeline (and Pipeline Step)

Every [Context](#context) produced by your [domain](#domain) is lazily piped through a Pipeline -- a sequential collection of Pipeline Steps. Pipeline Steps may either:
- Transform the Context somehow in a one-to-one fashion (e.g. mapping a value inside of the Context to another value).
- Transform the context into many other contexts in an one-to-many fashion (e.g. upscaling a Context, producing neighboring Contexts in the stream).
- Filter the Context out (meaning, not let it proceed to the next pipeline step).
- Do nothing, or just produce an invisible side-effect (e.g. run a shell command based on the Context's attached data).

The pipeline steps are executed in sequential fashion: the first one takes the input produced by [Domain](#domain), do it's processing, and passes its output to the second pipeline step. The second step takes the input from the first step, do its processing, and passes its output to the third... and so on... until the last one is reached, where the output is passed to the [Sink](#sink)s in a fan-out fashion.

#### Expressions and Functions
Pipeline Steps are capable of evaluating expressions in their arguments. Valid expressions includes primitives (either `null`, `string`, `i64`, `f64` or `bool`), string templates (e.g. `Hello, ${name}`), eval blocks (e.g. `$={another-expr}`) or function calling (`$={greet(first_name: "Nathan")}`).

### Sink

A workload can contain many sinks, which takes the output from the last [Pipeline Step](#pipeline). Different from the Pipeline Steps, Sinks are fan-out, meaning that if you have 5 different sinks, the pipeline output will be duplicated 5 times, in order to feed each one of them with the exact same input.

The intent of the sink is being the final consumer of the application, exporting the processed data (by the pipeline) into a meaningful format (e.g. a `geotiff` file).
