## Core Features
[x] - can ingest multiple files and also subdirectories
[x] - extracts audio and creates a transcription with hyperlinks to a source including "t= ..." for direct jumps to the referenced section in the video.
[x] - it uses a BJJ specific prompt for whisper for maximum correctness.
[x] - The "dictionary" of BJJ terms can be maintained via a config/text file.
[x] - it uses an LLM to further improve the transcription getting rid of common errors (e.g. "coast guard" for "closed guard").
[x] - it find chapters in the videos either via web scraping from the BJJ fanatics website or splash screen detection (or a combination of both).
[ ] - it inserts the chapter Info into the transcription.
[x] - it clusters the videos into series (same instructor, same topic) - **DONE via API**
[ ] - it creates a high-level and a technical summary via an LLM (it can use different LLM providers, mainly LM Studio)
[ ] - it creates a mermaid diagram for all series
[ ] - It prepares all the output for obsidian (.md)
[x] - it recognizes the "state" for each video (which steps have been succesfully completed) to not redo work, if started again. It does so by looking for the files created by the processing step (e.g. .mp3, .srt,...)
[x] - all prompts for LLMs can be edited in .txt files, so they can be changed without recompiling

## Infrastructure
[ ] clean up github repository
[ ] make remote usage of GPU (via Docker installation feasible

## UI/API Features (Completed June 2025)
[x] - REST API with video/series endpoints
[x] - Web UI with video library browser
[x] - Series management with auto-detection
[x] - Video details with chapter display
[x] - Corrections interface for series/products
[x] - Real-time status monitoring via WebSocket
[x] - Series creation/editing UI (frontend complete)

## Pending UI/Backend Integration
[ ] - Persist series edits/corrections to disk
[ ] - Implement series clustering in batch processing (currently API only)
[ ] - Connect UI process button to trigger video processing
[ ] - Add download functionality for results
[ ] - Implement retry mechanism for failed videos