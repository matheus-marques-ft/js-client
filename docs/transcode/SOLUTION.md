# Recording Transcode Module (Transcode)

## Overview

Transcodes JumpServer Guacamole-protocol session recordings (`.tar` archive files) into H.264 MP4 video files.

Supports native hardware/software encoding on three platforms:

| Platform | Encoder | Encoding method |
|------|--------|----------|
| macOS | VideoToolbox | Hardware-accelerated (GPU/ANE) |
| Linux | OpenH264 | Software encoding (CPU) |
| Windows | IMFSinkWriter | System-level pipeline (automatically picks a hardware/software encoder) |

## Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  Tauri Command: transcode_replays                                    │
│  (mod.rs)                                                            │
│  - Receives a list of tar file paths + output directory + user config │
│  - Extracts the tar -> pulls out replay.json + .part.gz              │
│  - gzip-decompresses to get the raw guacamole data                   │
│  - Calls transcode_to_mp4 to generate the video                      │
│  - Reports progress to the frontend via Tauri emit("transcode-progress") │
└──────────────────────────┬───────────────────────────────────────────┘
                           │
         ┌─────────────────┼──────────────────┐
         ▼                 ▼                  ▼
  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐
  │  parser.rs   │  │ renderer.rs  │  │  transcode.rs    │
  │              │  │              │  │                  │
  │ Guacamole    │  │ Multi-layer  │  │ Encode+mux       │
  │ protocol     │  │ canvas       │  │ pipeline         │
  │ parser       │  │ renderer     │  │ (per-platform)   │
  └──────────────┘  └──────────────┘  └───────┬──────────┘
                                              │
                           ┌──────────────────┼─────────────────────┐
                           ▼                  ▼                     ▼
                  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐
                  │  macOS / Linux │  │    Windows     │  │  encoder.rs      │
                  │                │  │                │  │                  │
                  │  Multi-thread  │  │  Single-thread │  │  Platform        │
                  │  chunk         │  │  Sink Writer   │  │  encoder         │
                  │  parallel      │  │  direct write  │  │  abstraction     │
                  │  encoding      │  │                │  │                  │
                  │  + manual MP4  │  │                │  │                  │
                  └────────────────┘  └────────────────┘  └──────────────────┘
```

### Module overview

| File | Responsibility |
|------|------|
| `mod.rs` | Tauri command entry point, tar/gzip extraction, progress events, resolution/bitrate/power config |
| `parser.rs` | Zero-copy parser for Guacamole's length-prefixed instruction format |
| `renderer.rs` | Maintains the multi-layer canvas, handles `size`/`img`/`blob`/`cfill`/`rect`/`copy` drawing instructions, composites RGB frames |
| `transcode.rs` | Encoding pipeline: frame timeline construction, render scheduling, scaling, encoding, MP4 muxing (per-platform) |
| `encoder.rs` | Platform encoder abstraction: macOS VideoToolbox / Linux OpenH264 / Windows IMFSinkWriter |

## Transcode Flow

### Shared stage (all platforms)

```
tar file
  ├─ <uuid>.replay.json     ← Session metadata (parsed with serde_json)
  └─ <uuid>.0.part.gz       ← gzip-compressed guacamole recording (can be multiple parts)
         │
         ▼ (decompressed with flate2, concatenated by part number)
    Raw guacamole instruction stream
         │
         ├─► scan_max_canvas_size    scans all size instructions to get the max canvas size
         ├─► parse_and_build_timeline samples the frame timeline at 100ms intervals
         │
         ▼
    FrameInfo list (timestamp + instruction_offset)
```

### macOS / Linux encoding path

```
FrameInfo list
    │
    ▼ (split into chunks, 50 frames per chunk)
    │
    ├─► chunk 0: encoded on the main thread -> extracts the canonical SPS/PPS
    │
    ├─► chunk 1..N: parallelized via std::thread::spawn
    │     each thread:
    │       1. restores parser + renderer state from instruction_offset
    │       2. composites frame by frame -> RGB
    │       3. scales to the target resolution with fast_image_resize
    │       4. converts RGB -> I420/YUV420
    │       5. encodes via VideoToolbox / OpenH264 -> NAL units
    │       6. frame dedup (FNV-1a hash); identical frames recorded via repeat_count
    │
    ▼ (collects all ChunkResults)
    │
    ▼ write_mp4_faststart:
       1. writes to a temp file: mdat (raw H.264 NAL stream + 4-byte length prefix)
       2. appends moov (manually built ISOBMFF box)
       3. rearranges into faststart layout: ftyp -> moov -> mdat
       4. deletes the temp file

Output: MP4 file (ftyp + moov + mdat)
```

### Windows encoding path

```
FrameInfo list
    │
    ▼ (single-threaded, single-pass rendering)
    │
    ├─► SinkWriterEncoder::new(output_path, w, h, bitrate, fps)
    │     1. CoInitializeEx + MFStartup
    │     2. MFCreateSinkWriterFromURL -> IMFSinkWriter
    │     3. configures the output type: H.264 + High Profile + bitrate
    │     4. AddStream -> gets stream_index
    │     5. configures the input type: RGB24 (BGR DIB byte order)
    │     6. BeginWriting
    │
    ├─► per-frame loop:
    │     1. parser replays up to the sync timestamp
    │     2. renderer.composite_into -> RGB frame
    │     3. scales to the target resolution with fast_image_resize
    │     4. frame dedup (FNV-1a hash)
    │     5. write_frame: RGB->BGR swap + vertical flip -> IMFMediaBuffer -> IMFSample -> WriteSample
    │        The Sink Writer automatically handles internally: BGR->NV12 color conversion -> H.264 encoding -> MP4 muxing
    │
    ├─► SinkWriterEncoder::finalize -> Finalize
    │
    ▼

Output: MP4 file (produced by the system's MP4 Muxer Sink)
```

## Key Design Points

### Frame sampling strategy

- Fixed 10fps output, one sample point every 100ms
- A first pass scans the guac instruction stream to build the `FrameInfo` timeline
- Capped at 600 frames (long recordings are automatically downsampled)

### Frame deduplication

- Computes an FNV-1a hash for each frame's RGB data (sampled at an 8-pixel stride)
- Consecutive identical frames aren't re-encoded; a `repeat_count` is recorded instead
- macOS/Linux: the repeat count is written into the MP4 `stts` box
- Windows: `WriteSample` is called repeatedly with the same frame data

### Resolution alignment

- Encoding width/height are aligned to a multiple of 16 (`& !15`)
- Ensures H.264 macroblocks (16×16) align cleanly, avoiding quality degradation from internal encoder padding

### Bitrate calculation

```rust
// Optimized for screen-recording content (text, icons, thin lines)
bitrate = pixels × 5 bps    // 5 bits per pixel
clamp(800 Kbps, 20 Mbps)
```

| Resolution | Bitrate |
|--------|------|
| 1920×1080 | 10.4 Mbps |
| 1280×768 | 4.9 Mbps |
| 1024×768 | 3.9 Mbps |
| 640×360 | 1.2 Mbps |

### Parallel encoding (macOS / Linux)

- The frame list is split into chunks (50 frames per chunk)
- The first chunk is encoded on the main thread to extract the canonical SPS/PPS
- The remaining chunks are distributed across `min(available CPUs × cpu_fraction, chunk count)` parallel threads
- Each thread creates its own encoder instance and independently parses guac instructions to restore render state

### Windows single-threaded pipeline

- IMFSinkWriter wraps the full encode+mux pipeline, automatically picking the best encoder internally (hardware preferred)
- Single-threaded per-frame render->write, no need to manage NAL/MP4 manually
- With hardware acceleration, encoder throughput is far higher than render speed, so single-threading isn't a bottleneck

### MP4 muxing (macOS / Linux)

- Manually builds ISOBMFF boxes: `ftyp` -> `moov` -> `mdat`
- Faststart layout: moov is placed before mdat, to support streaming playback
- Two-pass write: first writes to a temp file (mdat + moov), then rearranges into the final layout

## Platform Encoder Details

### macOS — VideoToolbox

```
RGB -> I420 (manual conversion, BT.601 matrix)
    -> shiguredo_video_toolbox crate
    -> Hardware Encoder (GPU/ANE)
    -> NAL units (Annex B)
    -> manually split + 4-byte length prefix
```

- Profile: Baseline + CAVLC
- GOP: 50 frames (5 seconds @10fps)
- Prioritizes encoding speed (`prioritize_encoding_speed_over_quality: true`)

### Linux — OpenH264

```
RGB -> YUV420 (via the openh264 crate's built-in RgbSliceU8 -> YUVBuffer)
    -> openh264::Encoder
    -> NAL units (Annex B)
    -> manually split via split_annex_b
    -> 4-byte length prefix
```

- Pure software encoding, CPU-intensive
- Supports multi-threaded chunk parallelism

### Windows — IMFSinkWriter

```
RGB -> BGR (per-pixel R/B swap, DIB byte order)
    -> vertical flip (top-down -> bottom-up)
    -> IMFMediaBuffer -> IMFSample
    -> IMFSinkWriter::WriteSample
    -> [internal to the system] Color Converter DSP (BGR->NV12)
    -> [internal to the system] H.264 Encoder MFT (hardware preferred: QSV/NVENC/AMF)
    -> [internal to the system] MP4 Muxer Sink
```

- Profile: High (CABAC entropy coding)
- The encoder is chosen automatically by the system, hardware preferred
- No need to manage NAL, SPS/PPS, or MP4 boxes manually

## Frontend Configuration

### User-selectable parameters

| Parameter | Options | Description | Platform impact |
|------|------|------|----------|
| `filename_style` | `original` / `friendly` / `friendly_uuid` | Output filename format | All platforms |
| `output_resolution` | `original` / `p1080` / `p720` / `p360` | Output resolution | All platforms |
| `transcode_power` | `auto` / `full` / `fast` / `medium` / `low` | CPU usage | Effective on macOS/Linux; fixed to `auto` on Windows |

### Progress events

Sent via Tauri `emit("transcode-progress", TranscodeProgress)`:

```typescript
interface TranscodeProgress {
  file: string // filename or session ID
  index: number // this file's index within the batch
  total: number // total number of files in the batch
  progress: number // 0–100
  message: string // status description
  success?: boolean // set on completion
  output?: string // output file path
  duration?: number // transcode duration (seconds)
  metadata?: ReplayMetadata // session metadata (sent with the first event)
}
```

## Dependencies

| crate | Purpose | Platform |
|-------|------|------|
| `flate2` | gzip-decompresses `.part.gz` | All platforms |
| `tar` | Extracts the `.tar` archive | All platforms |
| `image` | PNG/JPEG/WebP decoding (guacamole `blob` instruction) | All platforms |
| `fast_image_resize` | Bilinear-interpolation scaling | All platforms |
| `base64` | base64 decoding in the guacamole `blob` instruction | All platforms |
| `serde` / `serde_json` | replay.json parsing | All platforms |
| `tokio` | Async runtime + `spawn_blocking` | All platforms |
| `num_cpus` | Dynamically computes the parallel thread count | macOS / Linux |
| `shiguredo_video_toolbox` | VideoToolbox hardware encoder | macOS |
| `openh264` | OpenH264 software encoder | Linux |
| `windows` (MediaFoundation) | IMFSinkWriter system-level encoding pipeline | Windows |

## Known Limitations

- Only supports Guacamole-protocol recordings (RDP/VNC/SSH sessions going through a Guacamole gateway)
- Canvas size is taken from the max value across `size` instructions in the recording; mid-recording resolution switches aren't supported
- No audio track support (the guacamole `audio` instruction is ignored)
- Frame sampling is at a fixed interval, not event-driven, so static screens produce redundant frames (mitigated by frame dedup)
- The Windows path encodes single-threaded, with no chunk parallelism
