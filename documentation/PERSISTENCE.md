# Persistence Architecture - Redis-Style Resilience

## Overview

vectX uses a **Redis-style persistence model** that combines:
1. **In-Memory Storage**: All data structures live in memory for maximum speed
2. **Fork-Based Snapshots (RDB)**: Background snapshots using `fork()` - **completely non-blocking**
3. **Write-Ahead Log (WAL)**: Append-only log for durability (like Redis AOF)
4. **LMDB Backend**: Fast on-disk storage for point-in-time queries

## How It Works

### 1. In-Memory First (Like Redis)

All collections, points, HNSW indices, and BM25 indices are stored in memory:
- **Fast**: No disk I/O for reads/writes
- **Simple**: Direct memory access
- **Thread-Safe**: Uses `parking_lot::RwLock` for concurrent access

```rust
// All data in memory
collections: Arc<RwLock<HashMap<String, Arc<Collection>>>>
```

### 2. Fork-Based Snapshots (Exactly Like Redis RDB)

**How Redis Does It:**
1. Redis calls `fork()` to create a child process
2. Child process gets a copy of parent's memory (Copy-on-Write)
3. Child writes snapshot to disk
4. Parent continues serving requests (never blocks!)
5. Child exits when done

**How vectX Does It:**
```rust
// Fork child process
match unsafe { fork() } {
    Ok(ForkResult::Parent { child, .. }) => {
        // Parent: continue serving requests immediately
        // Background thread waits for child to complete
        return Ok(true); // Non-blocking!
    }
    Ok(ForkResult::Child) => {
        // Child: create snapshot and exit
        let snapshot = create_snapshot(collections);
        save_to_disk(snapshot);
        process::exit(0);
    }
}
```

**Key Benefits:**
- ✅ **Completely Non-Blocking**: Parent process never blocks, continues serving requests
- ✅ **Copy-on-Write**: OS handles memory sharing efficiently (only pages that change are copied)
- ✅ **Atomic Writes**: Writes to temp file, then renames (atomic operation)
- ✅ **Fast Recovery**: Load entire snapshot on startup
- ✅ **Zero Latency**: No impact on request handling

### 3. Write-Ahead Log (WAL) - Like Redis AOF

Every write operation is logged:
```rust
// Append to WAL
wal.append(&serialized_operation)?;
wal.sync()?; // Optional: fsync for durability
```

**On Startup:**
1. Load latest snapshot (fast recovery)
2. Replay WAL entries since snapshot (catch up to latest state)

### 4. LMDB Backend

For point-in-time queries and additional durability:
- Memory-mapped database
- ACID guarantees
- Fast random access

## Persistence Modes

### Mode 1: Snapshot Only (Default - Like Redis)
- Periodic background snapshots (every 5 minutes)
- Fast recovery
- May lose data between snapshots (acceptable for many use cases)

### Mode 2: WAL + Snapshot (More Durable)
- Every write logged to WAL
- Periodic snapshots
- Replay WAL on startup
- More durable, slightly slower writes

### Mode 3: WAL + Snapshot + fsync (Maximum Durability)
- WAL with `fsync()` on every write
- Maximum durability (like Redis `appendfsync always`)
- Slower writes (disk I/O)

## Configuration

```rust
// In StorageManager
save_interval: Some(Duration::from_secs(300)), // 5 minutes (like Redis)
```

## Recovery Process

1. **On Startup:**
   ```rust
   // Try to load snapshot
   if let Some(snapshot) = persistence.load_snapshot()? {
       // Restore all collections and points
       for collection in snapshot.collections {
           // Recreate collection
           // Restore all points
       }
   }
   
   // Replay WAL (if exists)
   // Apply any operations since snapshot
   ```

2. **Data Integrity:**
   - Snapshot is atomic (temp file + rename)
   - WAL entries are append-only
   - No corruption possible

## Performance Characteristics

| Operation | Time | Notes |
|----------|------|-------|
| Write (in-memory) | <1ms | No disk I/O |
| Write (with WAL) | 1-5ms | Depends on fsync |
| Background Save | **0ms blocking** | Fork-based, non-blocking |
| Snapshot Load | O(N) | N = number of points |
| WAL Replay | O(M) | M = number of operations |

## Comparison with Redis

| Feature | Redis | vectX |
|---------|-------|----------|
| In-Memory | ✅ | ✅ |
| Fork-based RDB | ✅ | ✅ |
| AOF/WAL | ✅ | ✅ |
| Copy-on-Write | ✅ | ✅ (OS handles) |
| Background Save | ✅ | ✅ |
| Atomic Writes | ✅ | ✅ |
| Non-Blocking | ✅ | ✅ |
| Periodic Saves | ✅ | ✅ |

## Resilience Features

1. **Non-Blocking Saves**: Fork-based, **never blocks main process**
2. **Atomic Writes**: Temp file + rename ensures consistency
3. **Automatic Recovery**: Loads snapshot + replays WAL on startup
4. **Periodic Saves**: Configurable interval (default: 5 minutes)
5. **Manual Saves**: `BGSAVE` command for on-demand snapshots
6. **Copy-on-Write**: OS efficiently handles memory sharing

## Example Usage

```rust
// Start server - automatically loads snapshot
let storage = StorageManager::new("./data")?;

// Writes are fast (in-memory)
collection.upsert(point)?;

// Background save happens automatically every 5 minutes
// Or trigger manually:
storage.bgsave()?; // Non-blocking (returns immediately)
storage.save()?;   // Blocking (for testing only)

// On restart, snapshot is automatically loaded
```

## How Fork Works (Technical Details)

1. **Fork() System Call**:
   - Creates exact copy of parent process
   - Both share same memory pages initially
   - OS marks pages as Copy-on-Write (COW)

2. **Copy-on-Write**:
   - If parent writes to a page → OS copies that page for parent
   - If child writes to a page → OS copies that page for child
   - Unchanged pages are shared (efficient!)

3. **Child Process**:
   - Has read-only access to parent's data
   - Can serialize snapshot without blocking parent
   - Exits when done

4. **Parent Process**:
   - Continues serving requests immediately
   - No blocking, no latency
   - Only pays cost of COW if it modifies data during snapshot

## Why This Approach is Resilient

1. **No Data Loss During Saves**: Parent continues operating normally
2. **Atomic Snapshots**: Temp file + rename ensures consistency
3. **Fast Recovery**: Single snapshot file loads quickly
4. **WAL for Durability**: Can replay operations if needed
5. **Proven Pattern**: Same approach as Redis (battle-tested)

## Future Enhancements

- [ ] WAL compaction (merge with snapshot)
- [ ] Incremental snapshots
- [ ] Replication (stream snapshots to replicas)
- [ ] Checkpointing (multiple snapshot versions)
- [ ] Background save progress tracking
