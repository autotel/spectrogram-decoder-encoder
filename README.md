# Spectrogram Decoder/Encoder v0.2

A high-quality bidirectional audio ↔ spectrogram converter with phase reconstruction, built in Rust.
Written by Claude Code

## What's New in v0.2

### 🎨 Higher Time Resolution (4x Width!)
- **Default hop size reduced from 512 to 128 samples**
- Creates images that are **4 times wider** for the same audio duration
- More precise control when editing spectrograms in image editors
- At 44.1kHz, each pixel represents ~2.9ms instead of ~11.6ms
- A 10-second audio file now generates ~3,448 pixels width instead of ~862 pixels

### ⚙️ Configuration System
- All encoding/decoding parameters now configurable via `spectrogram_config.toml`
- Config file is automatically created with sensible defaults on first run
- Self-healing: missing parameters are auto-added with defaults
- Live reload: modify config and reload without restarting the app

### 🎨 Phase Encoding Toggle
- **New option**: Choose between color (phase-encoded) or grayscale (magnitude-only) spectrograms
- **Color mode** (`use_phase_encoding = true`): Perfect audio reconstruction, colorful images
- **Grayscale mode** (`use_phase_encoding = false`): Easier visual editing, traditional spectrogram look
- Filename automatically indicates mode: `_PHASE` or `_MAG`

### 📊 Configuration Interface
- Toggle config display in the UI to see current settings
- One-click config reload after editing
- Direct "Open Config File" button
- Shows estimated output image width before conversion
- All settings including frequency scale now in config file

### ⚠️ Known Issues
- **Critical Bug**: When `use_phase_encoding = false` (grayscale/magnitude-only mode) in the config file, the image-to-audio conversion will currently **fail**.
  - **Workaround**: Always keep `use_phase_encoding = true` (color mode) if you need to convert images back to audio
  - Grayscale mode can be used for visual analysis only until this bug is fixed

## Features

- **Audio → Image**: Convert WAV files to spectrogram PNG images (color or grayscale)
- **Image → Audio**: Convert spectrogram images back to WAV audio with accurate phase reconstruction
- **Configurable parameters**: Full control via `spectrogram_config.toml` file
- **High time resolution**: 4x wider images by default for precise editing (hop size = 128)
- **Phase encoding options**: Color mode for perfect reconstruction or grayscale for easier visual editing
- **Logarithmic frequency mapping**: Musical note-based frequency scale (configurable)
- **Phase encoding**: HSV color space preserves both magnitude and phase information (when enabled)
- **High-quality reconstruction**: Pre-emphasis, dB scaling, and spectral holds minimize artifacts
- **Live configuration**: Reload settings without restarting the application
- Simple drag-and-drop or file picker interface
- Automatic output file naming with sample rate, scale mode, and encoding type embedded

## Building

Make sure you have Rust installed. Then:

```bash
cd spectrogram-decoder-encoder
cargo build --release
```

The executable will be at `target/release/spectrogram-decoder-encoder`

### Cross-platform Building

For Linux:
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

For Windows:
```bash
cargo build --release --target x86_64-pc-windows-gnu
```

For macOS:
```bash
cargo build --release --target x86_64-apple-darwin
```

## Configuration

On first run, a `spectrogram_config.toml` file is automatically created with sensible defaults. You can edit this file to customize encoding/decoding parameters.

### Configuration Parameters

```toml
# FFT window size - affects frequency resolution
# Larger = better frequency resolution, worse time resolution
# Must be a power of 2 (256, 512, 1024, 2048, 4096, 8192, etc.)
fft_size = 4096

# Hop size - distance between consecutive FFT windows
# Smaller = higher time resolution (wider images), more overlap
# Typical values: 64, 128, 256, 512
hop_size = 128

# Minimum frequency for logarithmic scale (Hz)
# Lower limit of analysis (human hearing starts at ~20 Hz)
min_freq = 20.0

# Dynamic range for visualization
db_min = -80.0  # Noise floor
db_max = 0.0    # Maximum level

# High-frequency boost (helps preserve high frequencies)
boost_start_freq = 1000.0      # Start boosting above this frequency
boost_db_per_octave = 6.0      # Amount of boost per octave

# Phase encoding (color vs grayscale)
use_phase_encoding = true      # true = color (perfect reconstruction), false = grayscale (easier editing)

# Frequency scale (logarithmic vs linear)
use_log_scale = true           # true = musical/note-based, false = linear/technical
```

### Adjusting Settings

1. Click "📝 Open Config File" in the app, or manually open `spectrogram_config.toml`
2. Edit values in your text editor
3. Save the file
4. Click "🔄 Reload Config" in the app
5. Convert your audio files with new settings

### Time Resolution Examples

At 44.1kHz sample rate, for a 10-second audio file:

| Hop Size | Overlap | ms/pixel | Image Width |
|----------|---------|----------|-------------|
| 512      | 87.5%   | ~11.6ms  | ~862 pixels       |
| 256      | 93.75%  | ~5.8ms   | ~1,724 pixels      |
| **128** (default) | **96.9%** | **~2.9ms** | **~3,448 pixels** |
| 64       | 98.4%   | ~1.45ms  | ~6,896 pixels      |

**The new default gives you 4x more pixels to work with when editing!**

## Usage

1. Run the application (on first run, `spectrogram_config.toml` is created automatically)
2. Either:
   - Click "Select File" to choose a file
   - Drag and drop a file onto the window
3. (Optional) Toggle "Show Config" to view current settings or adjust parameters in the config file
4. Click "Export" to convert

### Quick Example

```bash
# Build the project
cargo build --release

# Run the GUI
./target/release/spectrogram-decoder-encoder

# Or test with your own audio files:
# 1. Drop a WAV file → creates filename_SR44100_LOG_PHASE.png (or _LIN_PHASE.png, _LOG_MAG.png, etc.)
# 2. Drop that PNG back → creates filename.wav (sample rate and parameters extracted from filename)
```

### Supported Formats

- **Input audio**: WAV files (8/16/24/32-bit integer or 32-bit float, mono or stereo)
- **Input images**: PNG, JPG, JPEG
- **Output audio**: WAV (same sample rate as input, 16-bit, mono)
- **Output images**: PNG (HSV-encoded color spectrogram with phase, or grayscale magnitude-only)

### File Naming Convention

Output files include encoding parameters in the filename for automatic parameter detection:
- `filename_SR44100_LOG_PHASE.png`: Logarithmic scale, 44.1kHz, color (phase-encoded)
- `filename_SR44100_LOG_MAG.png`: Logarithmic scale, 44.1kHz, grayscale (magnitude-only)
- `filename_SR44100_LIN_PHASE.png`: Linear scale, 44.1kHz, color (phase-encoded)
- Legacy format `filename_SR44100_LOG.png` is assumed to be phase-encoded

## Configuration Examples

### Example 1: Music Production (Default)
```toml
fft_size = 4096           # Good frequency detail
hop_size = 128            # High time resolution
use_log_scale = true      # Musical/note-based spacing
use_phase_encoding = true # Perfect reconstruction
```
**Result**: 10s song → ~3,448 × 2,049 pixel color image

### Example 2: Sound Design (Maximum Precision)
```toml
fft_size = 2048           # Faster time response
hop_size = 64             # Extreme time resolution
use_log_scale = false     # Linear/technical analysis
use_phase_encoding = true # Perfect reconstruction
```
**Result**: 10s sound → ~6,896 × 1,025 pixel color image

### Example 3: Compact Files
```toml
fft_size = 4096
hop_size = 512            # Lower time resolution
use_log_scale = false     # Linear scale
use_phase_encoding = true # Keep color for now (grayscale has bug)
```
**Result**: 10s audio → ~862 × 2,049 pixel image (original v0.1 size)

### Example 4: Visual Analysis
```toml
fft_size = 4096
hop_size = 128
use_log_scale = true        # Musical spacing
use_phase_encoding = false  # Grayscale (⚠️ can't convert back currently)
```
**Result**: 10s audio → ~3,448 × 2,049 pixel grayscale image
**Use case**: Visual analysis only, not for round-trip conversion

## How It Works

### Audio → Image

1. **Preprocessing**
   - Stereo audio is converted to mono by averaging channels
   - Supports 8/16/24/32-bit integer and 32-bit float formats

2. **STFT Analysis**
   - FFT size: Configurable (default 4096 samples for high frequency resolution)
   - Hop size: Configurable (default 128 samples, 96.9% overlap for high time resolution)
   - Hann window applied to reduce spectral leakage

3. **Frequency Mapping** (Optional)
   - **Linear mode**: Evenly spaced frequencies (default)
   - **Logarithmic mode**: Musical note-based spacing (equal spacing per octave)
   - Log mapping uses exponential interpolation for smooth frequency warping
   - When using Log Mapping, the resulting audio quality is lower

4. **Pre-emphasis**
   - High frequencies boosted (configurable: default +6 dB/octave above 1 kHz)
   - Preserves detail in quieter high-frequency content
   - Counteracts natural 1/f spectral slope

5. **Encoding**
   - **Color mode** (`use_phase_encoding = true`):
     - **Hue**: Phase angle ([-π, π] → [0°, 360°])
     - **Saturation**: Phase hold flag (0 = hold phase from previous frame, 1 = new phase)
     - **Value**: Magnitude in dB scale ([-80 dB, 0 dB] → [0, 1])
   - **Grayscale mode** (`use_phase_encoding = false`):
     - **Brightness**: Magnitude in dB scale only (phase not encoded)

6. **Output**
   - PNG image with embedded metadata in filename
   - Format: `filename_SR{sample_rate}_{LOG|LIN}_{PHASE|MAG}.png`
   - X-axis = time, Y-axis = frequency (high at top)

### Image → Audio

1. **Metadata Extraction**
   - Sample rate, frequency scale mode, and phase encoding type read from filename
   - Determines FFT size from image height

2. **Decoding**
   - **Color mode** (phase-encoded):
     - **Hue** → phase angle
     - **Saturation** < 0.1 → phase held from previous frame (spectral hold)
     - **Value** → magnitude via inverse dB scale
   - **Grayscale mode** (magnitude-only):
     - **Brightness** → magnitude via inverse dB scale
     - Phase estimated using simple phase continuation
     - ⚠️ **Known bug**: Currently fails to decode (work in progress)

3. **De-emphasis**
   - Reverses the +6 dB/octave high-frequency boost
   - Restores original frequency balance

4. **Inverse Frequency Mapping**
   - If LOG mode: Maps logarithmic bins back to linear frequency bins
   - Uses interpolation to handle fractional bin positions

5. **Inverse STFT**
   - Constructs complex spectrum from magnitude and phase
   - Mirrors spectrum for negative frequencies (ensures real output)
   - Applies inverse FFT with overlap-add reconstruction
   - Hann window applied for smooth transitions

6. **Output**
   - Normalized WAV file to prevent clipping
   - Preserves original sample rate

## Choosing Phase Encoding Mode

### Use Color/Phase Mode (`use_phase_encoding = true`) when:
- You need perfect audio reconstruction
- You're doing precise audio editing or sound design
- You want to preserve all original audio information
- File size isn't a major concern

### Use Grayscale/Magnitude Mode (`use_phase_encoding = false`) when:
- You want easier visual editing in image editors
- You're removing or isolating specific frequencies
- You want traditional spectrogram appearance
- You prefer smaller file sizes (grayscale PNG compresses better)
- ⚠️ **Note**: Currently has a bug preventing image-to-audio conversion

## Advanced Features

### Logarithmic Frequency Mapping

When enabled, frequencies are mapped exponentially so that:
- Equal vertical distances = equal musical intervals
- Octaves have equal spacing (e.g., 110 Hz, 220 Hz, 440 Hz, 880 Hz)
- Better for music visualization and pitch-based editing
- Range: 20 Hz (bottom) to Nyquist frequency (top)

### Spectral Holds

The saturation channel encodes phase continuity:
- **Saturation = 1** (colored): Use this frame's phase
- **Saturation = 0** (grayscale): Continue phase from previous frame

This reduces artifacts when:
- Frequencies fade in/out
- Editing images manually (desaturate regions to create smooth holds)
- Dealing with noisy or quiet passages

### Image Editing Tips

You can manually edit the PNG images:
1. **Pitch shifting**: Shift image vertically (works best with LOG mode)
2. **Time stretching**: Resize image horizontally
3. **Filtering**: Paint over frequency regions
4. **Phase continuity**: Desaturate (make grayscale) to hold phase
5. **Dynamic control**: Adjust brightness to change loudness

Note: Keep hue intact when editing to preserve phase relationships

## Benefits of v0.2

### With 4x More Horizontal Pixels:
- **Precise timing**: Edit events down to ~3ms precision (vs ~12ms previously)
- **Smooth transitions**: Create gradual effects over more pixels
- **Detail work**: Remove or add frequency content with fine control
- **Visual clarity**: Easier to see and edit transient events
- **Less interpolation**: More data points = better reconstruction quality

### With Configuration System:
- **Full control**: Customize all parameters without modifying source code
- **Quick experimentation**: Try different settings easily
- **Project-specific**: Use different configs for different use cases
- **Self-documenting**: Config file includes extensive documentation

### Migration from v0.1

If upgrading from v0.1:
1. Replace all source files with the new versions
2. Update `Cargo.toml` with new dependencies
3. Run the app - it will create `spectrogram_config.toml`
4. Enjoy 4x wider spectrograms!

Old images created with v0.1 will still decode correctly (legacy filename format is supported).

## Possible Improvements

1. ~~**Configurable parameters**~~: ✅ **Implemented in v0.2** - Full config system via TOML file
2. **Fix grayscale mode bug**: Enable image-to-audio conversion for magnitude-only mode
3. **Multiple audio formats**: Add support for MP3, FLAC, OGG input
4. **Real-time preview**: Show waveform/spectrogram preview before export
5. **16-bit PNG support**: Use 16-bit grayscale for even better dynamic range
6. **Stereo preservation**: Encode left/right channels separately
7. **GPU acceleration**: Use compute shaders for faster STFT processing
8. **Batch processing**: Convert multiple files at once

## Quality Considerations

### Strengths
- Phase preservation enables high-quality reconstruction
- Pre-emphasis maintains high-frequency detail
- Large FFT size provides excellent frequency resolution
- High overlap (87.5%) ensures smooth time-domain reconstruction
- dB scale matches human hearing and audio engineering practices

### Limitations
- 8-bit PNG quantization limits dynamic range to ~80 dB
- Very transient sounds (clicks, snaps) may have artifacts due to time-frequency uncertainty
- Lossy image compression (JPEG) will degrade audio quality significantly
- Phase is most sensitive to quantization errors at low magnitudes

### Best Practices
1. Always use PNG (never JPEG) for lossless storage
2. Use color/phase encoding mode for perfect round-trip conversion
3. Use logarithmic scale for music, linear for technical analysis
4. Lower hop size values (64-128) for detailed time-domain editing
5. Higher FFT size values (8192) for better frequency resolution
6. Higher sample rates provide better high-frequency detail
7. Avoid resizing images - use native resolution
8. When editing color images, preserve the hue channel carefully
9. For now, avoid grayscale mode if you need to convert back to audio

## Technical Specifications

| Parameter | Default Value | Notes |
|-----------|---------------|-------|
| FFT Size | 4096 samples (configurable) | ~93 ms at 44.1 kHz |
| Hop Size | 128 samples (configurable) | ~2.9 ms at 44.1 kHz (4x improvement!) |
| Overlap | 96.9% | Very high overlap for quality |
| Window | Hann | Reduces spectral leakage |
| Frequency Resolution | ~10.8 Hz | At 44.1 kHz sample rate |
| Time Resolution | ~2.9 ms | Per frame (default) |
| Dynamic Range | 80 dB (configurable) | Encoded in 8-bit PNG |
| Phase Precision | ~1.4° | 256 levels in hue (color mode) |
| Log Scale Range | 20 Hz - Nyquist (configurable) | ~10 octaves at 44.1 kHz |
| Phase Encoding | Configurable | Color (perfect reconstruction) or grayscale (visual editing) |

## Dependencies

- `eframe/egui`: GUI framework
- `rfd`: File picker dialogs
- `hound`: WAV file I/O
- `rustfft`: Fast Fourier Transform
- `image`: Image reading/writing
- `serde/toml`: Configuration management
- `open`: Open config file in default editor

## License

Feel free to use and modify as needed!
