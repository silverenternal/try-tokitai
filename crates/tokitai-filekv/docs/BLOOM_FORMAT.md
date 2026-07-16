# Bloom Filter Persistence Format

## Overview

This document describes the on-disk format for persisted Bloom filter files in tokitai-filekv.

## File Location

Bloom filter files are stored in the index directory with the naming pattern:

```
bloom_{segment_id:06}.bin
```

Example: `bloom_000001.bin` for segment ID 1.

## File Format

Each bloom filter file uses the following binary layout (all integers are little-endian):

```
+------------------+------------------+------------------+------------------+
| Magic (4 bytes)  | Version (4B)     | NumKeys (8B)     | Key entries...   |
+------------------+------------------+------------------+------------------+
```

### Header

| Field    | Size     | Value                           | Description                      |
|----------|----------|---------------------------------|----------------------------------|
| Magic    | 4 bytes  | `0x424C4F4F` ("BLOO")           | Identifies the file as a bloom filter |
| Version  | 4 bytes  | Current: `1`                    | Format version number            |
| NumKeys  | 8 bytes  | Variable (u64)                  | Number of keys stored            |

### Key Entries

Each key is stored as a length-prefixed UTF-8 string:

```
+------------------+------------------+
| KeyLen (4 bytes) | Key (N bytes)    |
+------------------+------------------+
```

| Field  | Size     | Type   | Description              |
|--------|----------|--------|--------------------------|
| KeyLen | 4 bytes  | u32 LE | Length of the key in bytes |
| Key    | N bytes  | UTF-8  | The key string itself    |

## Constants

Defined in `src/core/types.rs`:

```rust
pub const BLOOM_MAGIC: u32 = 0x424C4F4F; // "BLOO"
pub const BLOOM_VERSION: u32 = 1;
```

## Serialization

The bloom filter is serialized by `save_bloom_filter_atomic()` in `src/bloom/manager.rs`:

1. Write magic number (4 bytes, little-endian)
2. Write version number (4 bytes, little-endian)
3. Write number of keys (8 bytes, little-endian)
4. For each key:
   - Write key length (4 bytes, little-endian)
   - Write key bytes (UTF-8 encoded)
5. Flush and sync the file
6. Atomically rename temp file to final path

## Deserialization

The bloom filter is deserialized by `load_bloom_filter()` in `src/bloom/manager.rs`:

1. Open and read the file
2. Validate magic number (must be `0x424C4F4F`)
3. Validate version number (must match `BLOOM_VERSION`)
4. Read number of keys (N)
5. For each of N keys:
   - Read key length
   - Read key bytes
6. Reconstruct the BloomFilter by inserting all keys

## Important Notes

### Keys-Only Storage

**The bitset is NOT persisted.** Only the list of keys is stored on disk. On load, a new `BloomFilter` is created and all keys are re-inserted to rebuild the bitset.

This means:
- Recovery time is O(N) where N = number of keys
- The false positive rate (FPR) may differ slightly from the original if the bloom was created with a different capacity than the default
- No bitmap compression or optimization is applied during rebuild

### Atomic Writes

Bloom filter files are written atomically using a write-then-rename pattern:
1. Write to `bloom_{id}.tmp`
2. Flush and sync
3. Rename to `bloom_{id}.bin`

This ensures that readers never see a partially-written file.

### Future Improvements

Potential optimizations to evaluate:

1. **Persist the bitset directly**: Store the compressed bitmap to avoid O(N) rebuild on load
2. **Add checksum**: Include a CRC32 or similar checksum for data integrity verification
3. **Key compression**: Use prefix compression for keys to reduce file size
4. **Block-based storage**: Group keys into blocks for faster partial loading
