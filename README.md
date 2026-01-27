# Spectrogram Converter

A simple bidirectional audio ↔ spectrogram converter built in Rust.
Written by Claude Code

## Features

- **Audio → Image**: Convert WAV files to spectrogram PNG images
- **Image → Audio**: Convert spectrogram images back to WAV audio
- Simple drag-and-drop or file picker interface
- Automatic output file naming (same location and name as input)

## Building

Make sure you have Rust installed. Then:

```bash
cd spectrogram-converter
cargo build --release
```

The executable will be at `target/release/spectrogram-converter`

## Usage

1. Run the application
2. Either:
   - Click "Select File" to choose a file
   - Drag and drop a file onto the window
3. Click "Export" to convert

### Supported Formats

- **Input audio**: WAV files
- **Input images**: PNG, JPG, JPEG
- **Output audio**: WAV (44.1kHz, 16-bit, mono)
- **Output images**: PNG (grayscale spectrogram)

## How It Works

### Audio → Image
- Performs Short-Time Fourier Transform (STFT)
- FFT size: 2048 samples
- Hop size: 512 samples
- Applies Hann window
- Converts magnitude to log scale for visualization
- Outputs grayscale PNG where:
  - X-axis = time
  - Y-axis = frequency (high frequencies at top)
  - Brightness = magnitude

### Image → Audio
- Reads pixel brightness as magnitude spectrum
- Reverses log scale transformation
- Uses phase gradient approach for resynthesis
- Performs inverse STFT with overlap-add
- Outputs normalized WAV file

## Improvements You Could Make

1. **Better phase reconstruction**: Implement Griffin-Lim algorithm for better resynthesis quality
2. **Configurable parameters**: Add UI controls for FFT size, hop size, sample rate
3. **Color spectrograms**: Use color to encode phase information
4. **Multiple audio formats**: Add support for MP3, FLAC, OGG via different libraries
5. **Stereo support**: Handle multi-channel audio
6. **Real-time preview**: Show waveform/spectrogram preview before export

## Dependencies

- `eframe/egui`: GUI framework
- `rfd`: File picker dialogs
- `hound`: WAV file I/O
- `rustfft`: Fast Fourier Transform
- `image`: Image reading/writing

## License

Feel free to use and modify as needed!
