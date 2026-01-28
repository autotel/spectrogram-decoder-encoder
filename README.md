# Spectrogram Decoder/Encoder

A high-quality bidirectional audio ↔ spectrogram converter with phase reconstruction, built in Rust.
Written by Claude Code

## Features

- **Audio → Image**: Convert WAV files to phase-encoded spectrogram PNG images
- **Image → Audio**: Convert spectrogram images back to WAV audio with accurate phase reconstruction
- **Logarithmic frequency mapping**: Musical note-based frequency scale (optional)
- **Phase encoding**: HSV color space preserves both magnitude and phase information
- **High-quality reconstruction**: Pre-emphasis, dB scaling, and spectral holds minimize artifacts
- Simple drag-and-drop or file picker interface
- Automatic output file naming with sample rate and scale mode embedded

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

## Usage

1. Run the application
2. Either:
   - Click "Select File" to choose a file
   - Drag and drop a file onto the window
3. **Optional**: Check "Logarithmic frequency scale" for musical note-based mapping
4. Click "Export" to convert

### Quick Example

```bash
# Build the project
cargo build --release

# Run the GUI
./target/release/spectrogram-decoder-encoder

# Or test with your own audio files:
# 1. Drop a WAV file → creates filename_SR44100_LIN.png or filename_SR44100_LOG.png
# 2. Drop that PNG back → creates filename.wav
```

### Supported Formats

- **Input audio**: WAV files (8/16/24/32-bit integer or 32-bit float, mono or stereo)
- **Input images**: PNG, JPG, JPEG
- **Output audio**: WAV (same sample rate as input, 16-bit, mono)
- **Output images**: PNG (HSV-encoded spectrogram with phase information)

## How It Works

### Audio → Image

1. **Preprocessing**
   - Stereo audio is converted to mono by averaging channels
   - Supports 8/16/24/32-bit integer and 32-bit float formats

2. **STFT Analysis**
   - FFT size: 4096 samples (high frequency resolution)
   - Hop size: 512 samples (87.5% overlap for smooth reconstruction)
   - Hann window applied to reduce spectral leakage

3. **Frequency Mapping** (Optional)
   - **Linear mode**: Evenly spaced frequencies (default)
   - **Logarithmic mode**: Musical note-based spacing (equal spacing per octave)
   - Log mapping uses exponential interpolation for smooth frequency warping
   - When using Log Mapping, the resulting audio quality is lower

4. **Pre-emphasis**
   - High frequencies boosted by +6 dB/octave above 1 kHz
   - Preserves detail in quieter high-frequency content
   - Counteracts natural 1/f spectral slope

5. **Encoding to HSV**
   - **Hue**: Phase angle ([-π, π] → [0°, 360°])
   - **Saturation**: Phase hold flag (0 = hold phase from previous frame, 1 = new phase)
   - **Value**: Magnitude in dB scale ([-80 dB, 0 dB] → [0, 1])

6. **Output**
   - PNG image with embedded metadata in filename
   - Format: `filename_SR{sample_rate}_LOG.png` or `filename_SR{sample_rate}_LIN.png`
   - X-axis = time, Y-axis = frequency (high at top)

### Image → Audio

1. **Metadata Extraction**
   - Sample rate and frequency scale mode read from filename
   - Determines FFT size from image height

2. **Decoding from HSV**
   - **Hue** → phase angle
   - **Saturation** < 0.1 → phase held from previous frame (spectral hold)
   - **Value** → magnitude via inverse dB scale

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

## Possible Improvements

1. **Configurable parameters**: Add UI controls for FFT size, hop size, dB range
2. **Multiple audio formats**: Add support for MP3, FLAC, OGG
3. **Real-time preview**: Show waveform/spectrogram preview before export
4. **16-bit PNG support**: Use 16-bit grayscale for even better dynamic range
5. **Stereo preservation**: Encode left/right channels separately
6. **GPU acceleration**: Use compute shaders for faster STFT processing

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
2. Use logarithmic mode for music, linear for analysis
3. Higher sample rates provide better high-frequency detail
4. Avoid resizing images - use native resolution
5. When editing, preserve the hue channel carefully

## Technical Specifications

| Parameter | Value | Notes |
|-----------|-------|-------|
| FFT Size | 4096 samples | ~93 ms at 44.1 kHz |
| Hop Size | 512 samples | ~12 ms at 44.1 kHz |
| Overlap | 87.5% | High overlap for quality |
| Window | Hann | Reduces spectral leakage |
| Frequency Resolution | ~10.8 Hz | At 44.1 kHz sample rate |
| Time Resolution | ~12 ms | Per frame |
| Dynamic Range | 80 dB | Encoded in 8-bit PNG |
| Phase Precision | ~1.4° | 256 levels in hue |
| Log Scale Range | 20 Hz - Nyquist | ~10 octaves at 44.1 kHz |

## Dependencies

- `eframe/egui`: GUI framework
- `rfd`: File picker dialogs
- `hound`: WAV file I/O
- `rustfft`: Fast Fourier Transform
- `image`: Image reading/writing

## License

Feel free to use and modify as needed!
