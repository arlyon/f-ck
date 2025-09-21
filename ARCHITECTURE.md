# Arch

The pipeline is composed of a few cached stages:

- the query DSL is parsed into the input files and query
- each input file is scanned to perform type resolution. this
  data is used to produce virtual columns addressed by their
  type which will be used when forwarding on to the next stage
- then, the fields that make up the primary key are resolved
  and indexed. this is only done when the primary key changes
- then, the query is converted into SQL and executed against
  the list of sources using polars.
