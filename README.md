# `zarrs-python`

```{warning}
The version of `zarr-python` we currently depend on is still in pre-release and this package is accordingly extremely experimental.  We cannot guarantee any stability or correctness at the moment, although we have tried to do extensive testing and make clear what we think we support and do not.
```

This project serves as a bridge between [`zarrs`](https://docs.rs/zarrs/latest/zarrs/) and [`zarr`](https://zarr.readthedocs.io/en/latest/index.html) via [`PyO3`](https://pyo3.rs/v0.22.3/).  The main goal of the project is to speed up i/o.

To use the project, simply install our package (which depends on `zarr-python>3.0.0b0`), and run:

```python
import zarr
import zarrs
zarr.config.set({"codec_pipeline.path": "zarrs.ZarrsCodecPipeline"})
```

You can then use your `zarr` as normal (with some caveats)!

## API

We export a `ZarrsCodecPipeline` class so that `zarr-python` can use the class but it is not meant to be instantiated and we do not guarantee the stability of its API beyond what is required so that `zarr-python` can use it.  Therefore, it is not documented here.  We also export two errors, `DiscontiguousArrayError` and `CollapsedDimensionError` that can be thrown in the process of converting to indexers that `zarrs` can understand (see below for more details).

### Configuration

`ZarrsCodecPipeline` options are exposed through `zarr.config`.

Standard `zarr.config` options control some functionality (see the defaults in the [config.py](https://github.com/zarr-developers/zarr-python/blob/main/src/zarr/core/config.py) of `zarr-python`):
- `threading.num_workers` (#threads if `None`): the maximum number of threads used internally by the `ZarrsCodecPipeline` (on the Rust side).
- `async.concurrency` (#threads if `None`): the maximum number of chunks stored/retrieved concurrently.
- `array.write_empty_chunks` ([`False`](https://docs.rs/zarrs/latest/zarrs/config/struct.Config.html#store-empty-chunks) if `None`): set whether or not to store empty chunks.
  - Pending [zarr-python #2429](https://github.com/zarr-developers/zarr-python/pull/2429)

The `ZarrsCodecPipeline` specific options are:
- `codec_pipeline.chunk_concurrent_minimum` ([4](https://docs.rs/zarrs/latest/zarrs/config/struct.Config.html#chunk-concurrent-minimum) if `None`): the minimum number of chunks retrieved/stored concurrently when balancing chunk/codec concurrency.
- `codec_pipeline.validate_checksums` ([`True`](https://docs.rs/zarrs/latest/zarrs/config/struct.Config.html#validate-checksums) if `None`): enable checksum validation (e.g. with the CRC32C codec).

For example:
```python
zarr.config.set({
    "threading.num_workers": None,
    "async.concurrency": None,
    "array.write_empty_chunks": False,
    "codec_pipeline": {
        "path": "zarrs.ZarrsCodecPipeline",
        "validate_checksums": True,
        "store_empty_chunks": False,
    }
})
```

## Concurrency

Concurrency can be classified into two types:
- chunk (outer) concurrency: the number of chunks retrieved/stored concurrently, and
- codec (inner) concurrency: the number of threads encoding/decoding a chunk.

`zarrs` and `zarrs-python` automatically balance chunk and codec concurrency based on factors such as the chunk size and the codecs.
Chunk concurrency is typically favored because:
- parallel encoding/decoding can have a high overhead with some codecs, especially with small chunks, and
- it is advantageous to retrieve/store multiple chunks concurrently, especially with high latency stores.

Sharded arrays are one of the main exceptions.
If encoding/decoding a shard (chunk) with many inner chunks, `zarrs` will favor codec concurrency over chunk concurrency.
However, the number of concurrent chunks will not drop below the `codec_pipeline.chunk_concurrent_minimum`, unless `threading.num_workers` is lower.

## Limitations

`zarrs-python` has several limitations to be aware of.

### Limited Store Support

`zarrs-python` only supports filesystem stores.

Support for more stores is planned in the near future - relevant [`zarrs` stores](https://docs.rs/zarrs/latest/zarrs/index.html#storage-support) just need to be wrapped.

### Supported Indexing Methods

We **do not** officially support the following indexing methods.  Some of these methods may error out, others may not:

1. Any discontinuous integer `np.ndarray` indexing for writes in 2D, and any integer `np.ndarray` indexing with dimensionality >=3 i.e.,
```python
arr[np.array([0, 5]), :] = ...
arr[np.array([...]), np.array([...]),  np.array([...])]
arr[np.array([...]), np.array([...]),  np.array([...])] = ...
```
2. `vindex` writes in 2D where both indexers are integer `np.ndarray` indices i.e.,
```python
arr[np.array([...]), np.array([...])] = ...
```
3. Ellipsis indexing.  We have tested some, but others fail even with `zarr-python`'s default codec pipeline.  Thus for now we advise proceeding with caution here.
```python
arr[0:10, ..., 0:5]
```

Otherwise, we believe that we support your indexing case: slices, ints, and all integer `np.ndarray` indices in 2D for reading, contiguous integer `np.ndarray` indices along one axis for writing etc.  Please file an issue if you believe we have more holes in our coverage than we are aware of or you wish to contribute!  For example, we have an [issue in zarrs for integer-array indexing](https://github.com/LDeakin/zarrs/issues/52) that would unblock a lot of these issues!

That being said, using non-contiguous integer `np.ndarray` indexing for reads may not be as fast as expected given the performance of other supported methods.  Until `zarrs` supports integer indexing, only fetching chunks is done in `rust` while indexing then occurs in `python`.

### No Partial Encoding

`zarrs` experimentally supports partial encoding (e.g. writing shard inner chunks incrementally), but this is not supported in `zarrs-python` yet.
