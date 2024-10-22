#!/usr/bin/env python3

import zarrs_python  # noqa: F401
import zarr
from zarr.storage import LocalStore, MemoryStore
import tempfile
import numpy as np

zarr.config.set(codec_pipeline={"path": "zarrs_python.ZarrsCodecPipeline",})

chunks = (2,2)
shape = (4,4)

tmp = tempfile.TemporaryDirectory()
arr = zarr.zeros(shape, store=LocalStore(root=tmp.name, mode='w'), chunks=chunks, dtype=np.uint8, codecs=[zarr.codecs.BytesCodec(), zarr.codecs.BloscCodec()])

# arr = zarr.zeros(shape, store=MemoryStore(mode='w'), chunks=chunks, dtype=np.uint8, codecs=[zarr.codecs.BytesCodec(), zarr.codecs.BloscCodec()])

print(arr[:])
# arr[:] = 42 # This is actually a special case that needs to be handled
arr[:] = np.arange(16).reshape(4,4)
print(arr[:])
