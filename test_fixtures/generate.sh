#!/bin/sh
# Generates one-second synthetic 440 Hz tones without third-party recordings.
# Requires FFmpeg with FLAC, AAC, Vorbis, Opus and MP3 encoders, plus speexenc.
set -eu
cd "$(dirname "$0")"
for spec in flac:flac aac:aac m4a:aac ogg:libvorbis opus:libopus mp3:libmp3lame; do
  extension=${spec%%:*}
  codec=${spec#*:}
  ffmpeg -hide_banner -loglevel error -f lavfi \
    -i sine=frequency=440:sample_rate=32000:duration=1 \
    -c:a "$codec" -y "tone.$extension"
done
speex_source=$(mktemp /tmp/tonhalle-speex-XXXXXX)
trap 'rm -f "$speex_source"' EXIT
ffmpeg -hide_banner -loglevel error -f lavfi \
  -i sine=frequency=440:sample_rate=32000:duration=1 \
  -f wav -y "$speex_source"
speexenc "$speex_source" tone.spx
