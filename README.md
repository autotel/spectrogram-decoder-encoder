# Spectrogram Converter
Vibe-coded w. Claude
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

### Color Mode (High Fidelity)
- **Filename**: `*_PHASE.png`
- **What you see**: Colorful spectrogram
- **Quality**: Near-perfect reconstruction (phase preserved)
- **Edit**: Hard (colors encode critical phase data)

### Grayscale Mode (Lower Fidelity)
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
# Larger FFT = better frequency resolution, worse time resolution
# Smaller FFT = better time resolution, worse frequency resolution
fft_size = 4096              # Must be power of 2 (256-16384)

# Smaller hop = better time resolution (more overlap, wider images)
# Larger hop = worse time resolution (less overlap, narrower images)
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
use_phase_encoding = true    # true = color with phase (best quality)
                             # false = grayscale magnitude only

# === Frequency Scale ===
use_log_scale = true         # true = musical (notes evenly spaced)
                             # false = technical (linear Hz)

# === Griffin-Lim (only for grayscale mode) ===
griffin_lim_iterations = 30  # More = better quality, slower (10-50)
```

## Quality Factors

| Setting | Effect on Reconstruction |
|---------|--------------------------|
| `use_phase_encoding = false` | **Lower fidelity** - Phase info lost, reconstructed with Griffin-Lim |
| `use_phase_encoding = true` | **Higher fidelity** - Phase preserved in color |
| Extreme `db_min` (e.g., -40 dB) | Quietest sounds clipped |
| Very low `db_min` (e.g., -80 dB) | All details captured |
| Small `fft_size` (< 1024) | Poor frequency resolution |
| Large `fft_size` (> 4096) | Excellent frequency resolution |
| Large `hop_size` (> 512) | Poor time resolution, artifacts |
| Small `hop_size` (< 256) | Excellent time resolution |

**For best quality**: Use color mode with `fft_size = 4096`, `hop_size = 128`, `db_min = -80`

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

```toml
# High-detail music visualization
fft_size = 4096      # Better frequency resolution
hop_size = 64        # Better time resolution
use_log_scale = true

# Fast editing (lower quality)
fft_size = 2048      # Lower frequency resolution
hop_size = 256       # Lower time resolution
use_phase_encoding = false

# Maximum quality (slow)
fft_size = 8192      # Excellent frequency resolution
hop_size = 128       # Excellent time resolution
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
