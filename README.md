# Video Audio Stream Processor

A lightweight Rust utility designed to automate the processing of video files in your local directory. It scans for supported video files, automatically identifies the highest-quality English audio track, converts it to stereo AAC, and strips unnecessary metadata and chapters-all while copying the video stream without any quality loss.

---

## Features

* **Automatic Audio Selection:** Scans all audio streams, filters for English tracks (`eng`, `en`), and picks the best one based on channel count and bitrate.
* **Lossless Video Copying:** Uses FFmpeg stream copy (`-c:v copy`) for the video track, ensuring zero quality degradation and fast processing times.
* **Audio Normalization:** Converts selected audio to AAC stereo (`2 channels`, `192 kbps`, `48 kHz`).
* **Clean Up:** Automatically strips global metadata and chapters to keep your files neat.
* **Safe Processing:** Processes videos using a temporary file and replaces the original only after a successful conversion, cleaning up after itself if any errors occur.

---

## Prerequisites

Before running the script, make sure you have the following installed on your system:

* **Rust**
* **FFmpeg & FFprobe:** Must be installed and added to your system's environment variables (PATH) so they can be executed from the command line.

---

## Supported Formats

The script automatically detects and processes files with the following extensions:
* `.mp4`
* `.mkv`
* `.avi`
* `.mov`
* `.m4v`
* `.webm`
---
