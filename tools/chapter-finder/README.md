# BJJ Fanatics Product Page Finder

A simple Python script to find BJJ Fanatics product pages from video filenames and automatically save them to a `product-pages.txt` file.

## Installation

```bash
pip install -r requirements.txt
```

## Usage

### Process a directory (finds all video files recursively)
```bash
python bjj_fanatics_finder.py "/path/to/video/directory"
```

### Process specific files
```bash
python bjj_fanatics_finder.py "JustStandUpbyCraigJones1.mp4" "ClosedGuardReintroducedbyAdamWardzinski1.mp4"
```

### Verbose output
```bash
python bjj_fanatics_finder.py "/path/to/videos" -v
```

**Note**: The script automatically searches subdirectories recursively and includes a 10-15 second delay between Google searches to avoid rate limiting.

## Example Output

```
Found 7 video files to process
ClosedGuardReintroducedbyAdamWardzinski1.mp4 -> https://bjjfanatics.com/products/closed-guard-reintroduced-by-adam-wardzinski
JustStandUpbyCraigJones1.mp4 -> https://bjjfanatics.com/products/just-stand-up-by-craig-jones
MikeyMusumeciVol1.mp4 -> https://bjjfanatics.com/products/the-knee-shield-system-part-1-attacking-far-side-by-mikey-musumeci
SystematicallyAttackingtheGuardTwoByGordonRyan1.mp4 -> https://bjjfanatics.com/products/systematically-attacking-the-guard-2-0-by-gordon-ryan
Product pages written to: /path/to/videos/product-pages.txt

Summary: 7/7 product pages found
```

## Output File

The script automatically creates a `product-pages.txt` file in the video directory with the format:
```
1→https://bjjfanatics.com/products/closed-guard-reintroduced-by-adam-wardzinski
2→https://bjjfanatics.com/products/just-stand-up-by-craig-jones
3→https://bjjfanatics.com/products/the-knee-shield-system-part-1-attacking-far-side-by-mikey-musumeci
4→https://bjjfanatics.com/products/systematically-attacking-the-guard-2-0-by-gordon-ryan
```

## How It Works

1. **File Discovery**: Automatically finds all video files (.mp4, .avi, .mkv, etc.) in directories and subdirectories recursively
2. **Filename Parsing**: Extracts instructor names and series titles from video filenames
3. **Google Search**: Uses Google search with `site:bjjfanatics.com` to find relevant product pages
4. **Rate Limiting**: Includes 10-15 second delays between searches to avoid Google rate limiting
5. **URL Filtering**: Returns the first valid BJJ Fanatics product URL found
6. **File Output**: Saves all found URLs to `product-pages.txt` in the video directory

## Important Notes

- **Rate Limiting**: Google may temporarily block requests if too many searches are performed quickly. The script includes automatic delays to minimize this.
- **Recursive Search**: The script will find video files in subdirectories automatically.
- **Supported Formats**: .mp4, .avi, .mkv, .mov, .wmv, .flv, .webm

## Requirements

- Python 3.6+
- Internet connection for Google search
- Dependencies: `googlesearch-python`, `requests`, `beautifulsoup4`