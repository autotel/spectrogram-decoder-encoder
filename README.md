# Spectrogram Converter

Vibe-coded using Claude

Bidirectional audio ↔ image converter. Turn sound into pictures, edit them, turn them back into sound.

## Quick Start

1. **Run the app**: `cargo run --release`
2. **Drop a file** or click "Select File"
3. **Click Export**

That's it. WAV becomes PNG, PNG becomes WAV.

## What Gets Encoded

The image encodes the **spectrum** of your audio:
- **X-axis** (width): Time
- **Y-axis** (height): Frequency (high frequencies at top)
- **Brightness**: How loud each frequency is
- **Color (hue)**: Phase information (timing of each frequency)

## Image Modes

### Color Mode (Lossless)
- **Filename**: `*_PHASE.png`
- **What you see**: Colorful spectrogram
- **Quality**: Perfect reconstruction (lossless)
- **Edit**: Hard (colors encode critical phase data)

### Grayscale Mode (Lossy)
- **Filename**: `*_MAG.png`
- **What you see**: Black & white spectrogram
- **Quality**: Good (uses Griffin-Lim to estimate missing phase)
- **Edit**: Easy (just brightness values)

## Filename Format

```
mysound_SR44100_LOG_PHASE.png
         ↑       ↑   ↑
         |       |   └─ PHASE=color (lossless) or MAG=grayscale (lossy)
         |       └───── LOG=logarithmic or LIN=linear frequency scale
         └───────────── Sample rate (needed for correct playback speed)
```

**Don't rename these files** - the decoder needs this info!

## Configuration

Create/edit `spectrogram_config.toml`:

```toml
# === Time/Frequency Resolution ===
# Trade-off: fft_size ↑ = better frequency resolution, worse time resolution
fft_size = 4096              # Must be power of 2 (256-16384)

# Trade-off: hop_size ↓ = better time resolution, wider images
hop_size = 128               # Smaller = more detail (try 64 for ultra-detail)

# === Frequency Range ===
min_freq = 20.0              # Lowest frequency for log scale (Hz)

# === Dynamic Range ===
db_min = -80.0               # Quietest sounds shown
db_max = 0.0                 # Loudest sounds (0 dB = full scale)

# === High-Frequency Boost ===
# Makes high frequencies visible (they're naturally quieter in images)
boost_start_freq = 1000.0    # Start boosting above this (Hz)
boost_db_per_octave = 6.0    # How much boost per octave

# === Encoding Mode ===
# IMPORTANT: This determines lossless vs lossy!
use_phase_encoding = true    # true = lossless color
                             # false = lossy grayscale

# === Frequency Scale ===
use_log_scale = true         # true = musical (notes evenly spaced)
                             # false = technical (linear Hz)

# === Griffin-Lim (only for grayscale mode) ===
griffin_lim_iterations = 30  # More = better quality, slower (10-50)
```

## What Makes It Lossy?

| Setting | Effect on Quality |
|---------|------------------|
| `use_phase_encoding = false` | **LOSSY** - Phase info discarded, reconstructed with Griffin-Lim |
| `use_phase_encoding = true` | **LOSSLESS** - Perfect reconstruction |
| Very low `db_min` (< -80 dB) | Lossless (captures all detail) |
| High `db_min` (e.g., -40 dB) | **LOSSY** - Quiet sounds clipped |
| Small `fft_size` (< 1024) | **LOSSY** - Poor frequency resolution |
| Large `hop_size` (> 512) | **LOSSY** - Poor time resolution |

**For lossless audio**: Use default config with `use_phase_encoding = true`

## Editing Spectrograms

### What You Can Do
- **Remove noise**: Paint black over unwanted frequencies
- **Isolate instruments**: Keep only certain frequency bands
- **Remove vocals**: Paint over the vocal frequency range
- **Time-stretch**: Resize width (makes audio slower/faster)
- **Pitch-shift**: Resize height (makes audio lower/higher)

### Tips
- Use **grayscale mode** (`use_phase_encoding = false`) for easier editing
- Edit with any image editor (Photoshop, GIMP, etc.)
- Black = silent, White = loud
- **Don't change the image height** (decoder expects exact FFT size)
- Save as PNG (JPEG compression will add artifacts)

## Frequency Scales

### Logarithmic (Musical)
- **Use when**: Working with music
- **What it does**: Notes are evenly spaced (like a piano keyboard)
- **Example**: C, C#, D are equally far apart in the image

### Linear (Technical)
- **Use when**: Scientific analysis, speech processing
- **What it does**: Frequencies are evenly spaced in Hz
- **Example**: 1000 Hz, 2000 Hz, 3000 Hz equally far apart

## Performance

| Operation | Color Mode | Grayscale Mode |
|-----------|-----------|----------------|
| Audio → Image | ~1 sec | ~1 sec |
| Image → Audio | ~1 sec | ~10-30 sec (Griffin-Lim) |

*Times for 10-second audio file*

## Examples

```bash
# High-detail music visualization
fft_size = 4096
hop_size = 64
use_log_scale = true

# Fast editing (lower quality)
fft_size = 2048
hop_size = 256
use_phase_encoding = false

# Maximum quality (slow)
fft_size = 8192
hop_size = 128
use_phase_encoding = true
griffin_lim_iterations = 50
```

## Troubleshooting

**Audio plays at wrong speed?**
- Don't rename files - sample rate is in filename

**Image looks all black?**
- Increase `db_max` or decrease `db_min`
- Audio might be very quiet

**Reconstructed audio sounds metallic?**
- Use color mode (`use_phase_encoding = true`)
- Or increase `griffin_lim_iterations`

**Image is too wide?**
- Increase `hop_size` (e.g., 256 or 512)

## Building

```bash
cargo build --release
```

Binary at `target/release/spectrogram-converter`

---

**Note**: For absolute lossless conversion, use color mode with default settings. Grayscale mode is always slightly lossy (but great for editing).