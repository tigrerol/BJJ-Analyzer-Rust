[ ]  - can ingest multiple files and also subdirectories
[ ]   - extracts audio and creates a transcription with hyperlinks to a source including "t= ..." for direct jumps to the referenced section in the video.
[ ] - it uses a BJJ specific prompt for whisper for maximum correctness.
[ ] - The "dictionary" of BJJ terms can be maintained via a config/text file.
[ ] - it uses an LLM to further improve the transcription getting rid of common errors (e.g. "coast guard" for "closed guard").
[ ] - it find chapters in the videos either via web scraping from the BJJ fanatics website or splash screen detection (or a combination of both).
[ ] - it inserts the chapter Info into the transcription.
[ ] - it clusters the videos into series (same instructor, same topic)
[ ] - it creates a high-level and a technical summary via an LLM (it can use different LLM providers, mainly LM Studio)
[ ] - it creates a mermaid diagram for all series
[ ] - It prepares all the output for obsidian (.md)
[ ] - it stores the "state" for each video (which steps have been succesfully completed) to not redo work, if started again.
[ ] - all prompts for LLMs can be edited in .txt files, so they can be changed without recompiling