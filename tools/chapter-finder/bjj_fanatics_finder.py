#!/usr/bin/env python3
"""
BJJ Fanatics Product Page Finder

Simple script to find BJJ Fanatics product pages from video filenames.
Extracted from the BJJ Video Analyzer project.
"""

import sys
import re
import logging
import time
import random
import requests
from pathlib import Path
from typing import Dict, List, Optional
try:
    from googlesearch import search
    GOOGLE_AVAILABLE = True
except ImportError:
    GOOGLE_AVAILABLE = False
    logging.warning("googlesearch-python not available")

try:
    from bs4 import BeautifulSoup
    BS4_AVAILABLE = True
except ImportError:
    BS4_AVAILABLE = False
    logging.warning("BeautifulSoup not available")

try:
    from duckduckgo_search import DDGS
    DUCKDUCKGO_AVAILABLE = True
except ImportError:
    DUCKDUCKGO_AVAILABLE = False
    logging.debug("duckduckgo-search not available")

try:
    from selenium import webdriver
    from selenium.webdriver.common.by import By
    from selenium.webdriver.support.ui import WebDriverWait
    from selenium.webdriver.support import expected_conditions as EC
    from selenium.webdriver.chrome.options import Options
    from selenium.common.exceptions import TimeoutException, WebDriverException
    SELENIUM_AVAILABLE = True
except ImportError:
    SELENIUM_AVAILABLE = False
    logging.debug("selenium not available")


def setup_logging(verbose: bool = False):
    """Setup basic logging."""
    level = logging.DEBUG if verbose else logging.INFO
    logging.basicConfig(
        level=level,
        format='%(levelname)s: %(message)s'
    )


def parse_video_filename(filename: str) -> Dict[str, List[str]]:
    """Parse video filename to extract instructor name and series title.
    
    Args:
        filename: Video filename (e.g., "JustStandUpbyCraigJones1.mp4")
        
    Returns:
        Dictionary with 'instructor' and 'series' word lists
    """
    # Remove file extension and common patterns
    clean_name = re.sub(r'\d+\.mp4$', '', filename)  # Remove trailing numbers and .mp4
    clean_name = re.sub(r'[_\-]', ' ', clean_name)    # Replace underscores and dashes with spaces
    
    # Handle patterns like "Reintroducedby" -> "Reintroduced by"
    clean_name = re.sub(r'(by)([A-Z])', r' \1 \2', clean_name)  # "byAdam" -> " by Adam"
    clean_name = re.sub(r'([a-z])(by)([A-Z])', r'\1 \2 \3', clean_name)  # "Reintroducedby" -> "Reintroduced by"
    
    # Handle common word boundaries that might not have capitals
    # "Blocksof" -> "Blocks of", "Guardby" -> "Guard by"
    clean_name = re.sub(r'([a-z])(of)([A-Z])', r'\1 \2 \3', clean_name)
    clean_name = re.sub(r'([a-z])(to)([A-Z])', r'\1 \2 \3', clean_name)
    clean_name = re.sub(r'([a-z])(the)([A-Z])', r'\1 \2 \3', clean_name)
    
    # Split on capital letters to separate words
    words = re.findall(r'[A-Z][a-z]*|[a-z]+', clean_name)
    
    # Try to find "by" indicator to split instructor and series
    by_index = -1
    for i, word in enumerate(words):
        if word.lower() == 'by':
            by_index = i
            break
    
    instructor_words = []
    series_words = []
    
    if by_index != -1:
        # Found "by" - everything after "by" is instructor, everything before is series
        series_words = [word for word in words[:by_index] if word.lower() not in ['the', 'of', 'and', 'or', 'in', 'on', 'at', 'to', 'for', 'with', 'from']]
        instructor_words = [word for word in words[by_index+1:] if word.lower() not in ['the', 'of', 'and', 'or', 'in', 'on', 'at', 'to', 'for', 'with', 'from']]
    else:
        # No "by" found - use heuristics
        # Filter out common stop words first
        filtered_words = [word for word in words if word.lower() not in ['by', 'the', 'of', 'and', 'or', 'in', 'on', 'at', 'to', 'for', 'with', 'from']]
        
        # Common BJJ technique/series indicators (these are more likely to be in series titles)
        series_indicators = ['guard', 'control', 'submission', 'sweep', 'escape', 'pass', 'position', 'mount', 'choke', 'lock', 'system', 'fundamentals', 'basics', 'blocks', 'building']
        
        # Try to identify instructor name (usually last 2-3 words if no obvious series indicators)
        if len(filtered_words) >= 4:
            # Look for series indicators in the first part
            has_series_indicators = any(word.lower() in series_indicators for word in filtered_words[:len(filtered_words)//2])
            
            if has_series_indicators:
                # Likely pattern: [Series Title] [Instructor Name]
                # Take last 2-3 words as instructor
                instructor_words = filtered_words[-2:] if len(filtered_words) >= 2 else filtered_words[-1:]
                series_words = filtered_words[:-len(instructor_words)]
            else:
                # Fallback: treat first 2-3 words as instructor (original logic)
                instructor_words = filtered_words[:3] if len(filtered_words) >= 3 else filtered_words[:2]
                series_words = filtered_words[len(instructor_words):]
        else:
            # Short filename - use original logic
            instructor_words = filtered_words[:2] if len(filtered_words) >= 2 else filtered_words
            series_words = filtered_words[len(instructor_words):]
    
    # Clean up empty lists
    instructor_words = [word for word in instructor_words if word]
    series_words = [word for word in series_words if word]
    all_words = [word for word in words if word.lower() not in ['by', 'the', 'of', 'and', 'or', 'in', 'on', 'at', 'to', 'for', 'with', 'from']]
    
    return {
        'instructor': instructor_words,
        'series': series_words,
        'all_words': all_words
    }


def search_bjj_fanatics_selenium(search_terms: Dict[str, List[str]]) -> Optional[str]:
    """Search BJJ Fanatics using Selenium to handle JavaScript-rendered content.
    
    Args:
        search_terms: Dictionary with instructor and series terms
        
    Returns:
        BJJ Fanatics product URL if found, None otherwise
    """
    if not SELENIUM_AVAILABLE:
        logging.debug("Selenium not available for browser automation")
        return None
        
    driver = None
    try:
        # Try multiple search query strategies - INSTRUCTOR-FOCUSED
        search_queries = []
        
        # Strategy 1: Instructor name only (most reliable)
        if search_terms['instructor']:
            instructor_query = ' '.join(search_terms['instructor'])
            search_queries.append(f'"{instructor_query}"')
        
        # Strategy 2: Instructor + key series words (refined)
        if search_terms['instructor'] and search_terms['series']:
            instructor_query = ' '.join(search_terms['instructor'])
            key_series = ' '.join(search_terms['series'][:2])  # Just 2 key words
            search_queries.append(f'"{instructor_query}" {key_series}')
        
        # Strategy 3: Instructor + full series (fallback)
        if search_terms['instructor'] and search_terms['series']:
            instructor_query = ' '.join(search_terms['instructor'])
            series_query = ' '.join(search_terms['series'])
            search_queries.append(f'"{instructor_query}" {series_query}')
        
        # Set up Chrome options for headless browsing
        chrome_options = Options()
        chrome_options.add_argument("--headless")
        chrome_options.add_argument("--no-sandbox")
        chrome_options.add_argument("--disable-dev-shm-usage")
        chrome_options.add_argument("--disable-gpu")
        chrome_options.add_argument("--window-size=1920,1080")
        chrome_options.add_argument("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        
        # Create driver
        driver = webdriver.Chrome(options=chrome_options)
        driver.set_page_load_timeout(30)
        
        # Try each search query until we find results
        for search_query in search_queries:
            logging.info(f"Selenium BJJ Fanatics search query: '{search_query}'")
            
            try:
                # Navigate to BJJ Fanatics search page
                search_url = f"https://bjjfanatics.com/search?q={requests.utils.quote(search_query)}"
                logging.debug(f"Navigating to: {search_url}")
                driver.get(search_url)
                
                # Wait for search results to load (JavaScript)
                wait = WebDriverWait(driver, 15)
                
                # Try different selectors for product links
                product_selectors = [
                    'a[href*="/products/"]',
                    '.product-item a',
                    '.grid-product__link',
                    '.product-card a',
                    '.product-link',
                    '.grid__item a'
                ]
                
                product_links = []
                
                for selector in product_selectors:
                    try:
                        # Wait for elements to be present
                        elements = wait.until(EC.presence_of_all_elements_located((By.CSS_SELECTOR, selector)))
                        
                        for element in elements[:10]:  # Limit to first 10 results
                            try:
                                href = element.get_attribute('href')
                                if href and '/products/' in href and 'bjjfanatics.com' in href:
                                    # Filter out obviously wrong results - these are generic/promotional products
                                    invalid_products = ['atos2025', 'gift-card', 'retreat', 'insiders-club', 'vip-retreat', 'fanatics-retreat']
                                    if not any(invalid in href for invalid in invalid_products):
                                        product_links.append(href)
                            except:
                                continue
                        
                        if product_links:
                            break  # Found products with this selector
                            
                    except TimeoutException:
                        logging.debug(f"Selector '{selector}' timed out")
                        continue
                
                # Remove duplicates
                product_links = list(dict.fromkeys(product_links))
                
                if product_links:
                    logging.info(f"Selenium search found {len(product_links)} products with query: '{search_query}'")
                    first_product = product_links[0]
                    logging.info(f"Found BJJ Fanatics product via Selenium: {first_product}")
                    return first_product
                else:
                    logging.debug(f"No valid products found with query: '{search_query}'")
                    
            except Exception as e:
                logging.debug(f"Search query '{search_query}' failed: {e}")
                continue
        
        logging.warning("No products found via Selenium BJJ Fanatics search")
        return None
            
    except WebDriverException as e:
        logging.error(f"Selenium WebDriver error: {e}")
        return None
    except Exception as e:
        logging.error(f"Selenium BJJ Fanatics search failed: {e}")
        return None
    finally:
        if driver:
            try:
                driver.quit()
            except:
                pass


def search_bjj_fanatics_direct(search_terms: Dict[str, List[str]]) -> Optional[str]:
    """Search BJJ Fanatics directly - fallback to simple requests if Selenium fails.
    
    Args:
        search_terms: Dictionary with instructor and series terms
        
    Returns:
        BJJ Fanatics product URL if found, None otherwise
    """
    # Try Selenium first for JavaScript handling
    if SELENIUM_AVAILABLE:
        result = search_bjj_fanatics_selenium(search_terms)
        if result:
            return result
    
    # Fallback: simple requests (may not work well due to JavaScript)
    logging.debug("Selenium failed, trying simple requests fallback")
    
    if not BS4_AVAILABLE:
        logging.warning("BeautifulSoup not available for direct search")
        return None
        
    try:
        # Build search query from terms
        query_parts = []
        
        # Add instructor name
        if search_terms['instructor']:
            instructor_query = ' '.join(search_terms['instructor'])
            query_parts.append(instructor_query)
        
        # Add series/technique terms (use more words for better matching)
        if search_terms['series']:
            series_query = ' '.join(search_terms['series'][:4])  # First 4 words for better context
            query_parts.append(series_query)
        
        # Create search query
        search_query = ' '.join(query_parts)
        logging.debug(f"Fallback BJJ Fanatics search query: '{search_query}'")
        
        search_url = f"https://bjjfanatics.com/search?q={requests.utils.quote(search_query)}"
        
        headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
        }
        
        response = requests.get(search_url, headers=headers, timeout=10)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, 'html.parser')
        
        # Look for product links
        product_links = []
        selectors = ['a[href*="/products/"]']
        
        for selector in selectors:
            links = soup.select(selector)
            for link in links:
                href = link.get('href')
                if href and '/products/' in href:
                    if href.startswith('/'):
                        href = f"https://bjjfanatics.com{href}"
                    product_links.append(href)
        
        # Remove duplicates
        product_links = list(dict.fromkeys(product_links))
        
        if product_links:
            logging.debug(f"Fallback search found {len(product_links)} products")
            first_product = product_links[0]
            logging.debug(f"Found BJJ Fanatics product via fallback: {first_product}")
            return first_product
        
        logging.warning("No products found via fallback search")
        return None
        
    except Exception as e:
        logging.error(f"Fallback BJJ Fanatics search failed: {e}")
        return None


def search_bjj_fanatics_google(search_terms: Dict[str, List[str]]) -> Optional[str]:
    """Search Google for BJJ Fanatics product page.
    
    Args:
        search_terms: Dictionary with instructor and series terms
        
    Returns:
        BJJ Fanatics product URL if found, None otherwise
    """
    try:
        # Build Google search query
        query_parts = []
        
        # Add instructor name with high priority
        if search_terms['instructor']:
            instructor_query = ' '.join(search_terms['instructor'])
            query_parts.append(f'"{instructor_query}"')  # Use quotes for exact match
        
        # Add series/technique terms (use more words for better matching)
        if search_terms['series']:
            series_query = ' '.join(search_terms['series'][:4])  # First 4 words for better context
            query_parts.append(series_query)
        
        # Construct final Google query
        google_query = f"site:bjjfanatics.com {' '.join(query_parts)}"
        logging.info(f"Google search query: {google_query}")
        
        # Perform Google search
        search_results = list(search(google_query, num_results=5, lang="en"))
        logging.info(f"Google search returned {len(search_results)} results")
        
        for i, url in enumerate(search_results):
            logging.debug(f"Google result {i+1}: {url}")
            
            # Filter for product pages
            if '/products/' in url and 'bjjfanatics.com' in url:
                logging.info(f"Found BJJ Fanatics product: {url}")
                return url
        
        # If no direct product links, try the first BJJ Fanatics result
        bjj_results = [url for url in search_results if 'bjjfanatics.com' in url]
        if bjj_results:
            logging.info(f"Using first BJJ Fanatics result: {bjj_results[0]}")
            return bjj_results[0]
        
        logging.warning("No suitable BJJ Fanatics products found")
        return None
        
    except Exception as e:
        logging.error(f"Google search failed: {e}")
        return None


def search_bjj_fanatics_duckduckgo(search_terms: Dict[str, List[str]]) -> Optional[str]:
    """Search DuckDuckGo for BJJ Fanatics product page.
    
    Args:
        search_terms: Dictionary with instructor and series terms
        
    Returns:
        BJJ Fanatics product URL if found, None otherwise
    """
    if not DUCKDUCKGO_AVAILABLE:
        logging.debug("DuckDuckGo search not available")
        return None
        
    try:
        # INSTRUCTOR-FOCUSED search strategies
        search_queries = []
        
        # Strategy 1: Instructor only (most reliable)
        if search_terms['instructor']:
            instructor_query = ' '.join(search_terms['instructor'])
            search_queries.append(f'site:bjjfanatics.com "{instructor_query}"')
        
        # Strategy 2: Instructor + key series terms
        if search_terms['instructor'] and search_terms['series']:
            instructor_query = ' '.join(search_terms['instructor'])
            key_series = ' '.join(search_terms['series'][:2])
            search_queries.append(f'site:bjjfanatics.com "{instructor_query}" {key_series}')
        
        # Try each search query
        for ddg_query in search_queries:
            logging.info(f"DuckDuckGo search query: {ddg_query}")
            
            try:
                # Perform DuckDuckGo search
                with DDGS() as ddgs:
                    search_results = list(ddgs.text(ddg_query, max_results=5))
                    logging.info(f"DuckDuckGo search returned {len(search_results)} results")
                    
                    for i, result in enumerate(search_results):
                        url = result.get('href', '')
                        logging.debug(f"DuckDuckGo result {i+1}: {url}")
                        
                        # Filter for product pages
                        if '/products/' in url and 'bjjfanatics.com' in url:
                            # Skip obviously wrong results
                            if 'atos2025' not in url and 'gift-card' not in url and 'retreat' not in url and 'insiders-club' not in url:
                                logging.info(f"Found BJJ Fanatics product via DuckDuckGo: {url}")
                                return url
                    
                    # If no filtered product links, try the first BJJ Fanatics result
                    bjj_results = [r.get('href', '') for r in search_results if 'bjjfanatics.com' in r.get('href', '') and '/products/' in r.get('href', '')]
                    if bjj_results:
                        first_result = bjj_results[0]
                        if 'atos2025' not in first_result and 'gift-card' not in first_result and 'retreat' not in first_result:
                            logging.info(f"Using first filtered BJJ Fanatics result: {first_result}")
                            return first_result
                
            except Exception as e:
                logging.debug(f"DuckDuckGo query '{ddg_query}' failed: {e}")
                continue
            
        logging.warning("No suitable BJJ Fanatics products found via DuckDuckGo")
        return None
        
    except Exception as e:
        logging.error(f"DuckDuckGo search failed: {e}")
        return None


def clean_product_url(product_url: str) -> str:
    """Clean product URL by removing tracking parameters and normalizing.
    
    Args:
        product_url: Raw product URL possibly with tracking parameters
        
    Returns:
        Cleaned URL without parameters
    """
    if not product_url:
        return ""
    
    # Remove tracking parameters (everything after ? or #)
    clean_url = product_url.split('?')[0].split('#')[0]
    return clean_url.lower()


def validate_product_url(product_url: str, search_terms: Dict[str, List[str]]) -> bool:
    """Validate that the product URL contains at least part of the instructor's name.
    
    Args:
        product_url: The product page URL to validate
        search_terms: Dictionary with instructor and series terms
        
    Returns:
        True if URL is valid (contains instructor name), False otherwise
    """
    if not product_url or not search_terms.get('instructor'):
        return False
    
    # Clean URL and convert to lowercase for case-insensitive matching
    url_lower = clean_product_url(product_url)
    
    # Check if any part of the instructor's name appears in the URL
    for name_part in search_terms['instructor']:
        if len(name_part) >= 3 and name_part.lower() in url_lower:
            logging.debug(f"URL validation passed: '{name_part}' found in {product_url}")
            return True
    
    # Check for common name variations (first/last name combinations)
    instructor_full = ' '.join(search_terms['instructor']).lower()
    instructor_parts = [part.lower() for part in search_terms['instructor'] if len(part) >= 3]
    
    # Check if first and last name appear together (with hyphens)
    if len(instructor_parts) >= 2:
        # Try "firstname-lastname" pattern
        name_combo = '-'.join(instructor_parts[:2])
        if name_combo in url_lower:
            logging.debug(f"URL validation passed: '{name_combo}' found in {product_url}")
            return True
        
        # Try "lastname-firstname" pattern  
        name_combo = '-'.join(instructor_parts[:2][::-1])
        if name_combo in url_lower:
            logging.debug(f"URL validation passed: '{name_combo}' found in {product_url}")
            return True
    
    logging.debug(f"URL validation failed: No instructor name found in {product_url}")
    logging.debug(f"Expected instructor: {search_terms['instructor']}")
    return False


def find_product_page(filename: str) -> Optional[str]:
    """Find BJJ Fanatics product page for a video filename.
    
    Args:
        filename: Video filename
        
    Returns:
        Product page URL if found and validated, None otherwise
    """
    logging.info(f"Processing filename: {filename}")
    
    # Parse filename to extract search terms
    search_terms = parse_video_filename(filename)
    logging.debug(f"Parsed search terms: {search_terms}")
    
    # Try multiple search methods in order of preference
    product_url = None
    
    # Method 1: DuckDuckGo search (reliable, no rate limits)
    if DUCKDUCKGO_AVAILABLE:
        logging.info("Method 1: Trying DuckDuckGo search...")
        product_url = search_bjj_fanatics_duckduckgo(search_terms)
        if product_url and validate_product_url(product_url, search_terms):
            logging.info(f"Found and validated product page: {product_url}")
            return product_url
        elif product_url:
            logging.warning(f"Found product but validation failed, continuing search: {product_url}")
            product_url = None
    
    # Method 2: Direct BJJ Fanatics search with Selenium (handles JavaScript)
    if not product_url:
        logging.info("Method 2: Trying direct BJJ Fanatics search...")
        product_url = search_bjj_fanatics_direct(search_terms)
        if product_url and validate_product_url(product_url, search_terms):
            logging.info(f"Found and validated product page: {product_url}")
            return product_url
        elif product_url:
            logging.warning(f"Found product but validation failed, continuing search: {product_url}")
            product_url = None
    
    # Method 3: Google search (fallback, may have rate limits)
    if not product_url and GOOGLE_AVAILABLE:
        logging.info("Method 3: Trying Google search as last resort...")
        product_url = search_bjj_fanatics_google(search_terms)
        if product_url and validate_product_url(product_url, search_terms):
            logging.info(f"Found and validated product page: {product_url}")
            return product_url
        elif product_url:
            logging.warning(f"Found product but validation failed: {product_url}")
            product_url = None
    
    logging.warning(f"No valid product page found for: {filename}")
    return None


def get_output_file_path(video_paths: List[str]) -> Path:
    """Get the path for product-pages.txt based on video file locations.
    
    Args:
        video_paths: List of video file paths
        
    Returns:
        Path to product-pages.txt file
    """
    # Determine the common directory of all video files
    if len(video_paths) == 1:
        # Single file - use its directory
        video_dir = Path(video_paths[0]).parent
    else:
        # Multiple files - try to find common parent directory
        paths = [Path(p) for p in video_paths]
        try:
            # Find common parent directory
            common_parts = Path(paths[0]).parts
            for path in paths[1:]:
                parts = path.parts
                common_parts = common_parts[:len([i for i, (a, b) in enumerate(zip(common_parts, parts)) if a == b])]
            
            if common_parts:
                video_dir = Path(*common_parts)
            else:
                video_dir = Path.cwd()  # Fallback to current directory
        except:
            video_dir = Path.cwd()  # Fallback to current directory
    
    return video_dir / "product-pages.txt"


def load_existing_results(video_paths: List[str]) -> Dict[str, str]:
    """Load existing results from product-pages.txt if it exists.
    
    Args:
        video_paths: List of video file paths to determine output location
        
    Returns:
        Dictionary mapping URLs to URLs (for consistency with matching logic)
    """
    output_file = get_output_file_path(video_paths)
    existing_results = {}
    
    if not output_file.exists():
        logging.debug(f"No existing product-pages.txt found at {output_file}")
        return existing_results
    
    try:
        with open(output_file, 'r') as f:
            lines = f.readlines()
        
        logging.info(f"Loading existing results from {output_file}")
        
        for line in lines:
            line = line.strip()
            if line and line.startswith('http'):
                # New format: just the URL on each line
                existing_results[line] = line
            elif '→' in line:
                # Legacy format: "1→https://bjjfanatics.com/products/..."
                parts = line.split('→', 1)
                if len(parts) == 2:
                    url = parts[1].strip()
                    existing_results[url] = url
        
        logging.info(f"Loaded {len(existing_results)} existing results")
        return existing_results
        
    except Exception as e:
        logging.error(f"Failed to load existing results: {e}")
        return {}


def extract_instructor_from_url(product_url: str) -> List[str]:
    """Extract instructor name from BJJ Fanatics product URL.
    
    Args:
        product_url: BJJ Fanatics product URL
        
    Returns:
        List of instructor name parts found in URL
    """
    if not product_url or '/products/' not in product_url:
        return []
    
    # Clean URL and extract product name part
    clean_url = clean_product_url(product_url)
    product_part = clean_url.split('/products/')[-1]
    
    # Split product name by hyphens and extract potential instructor names
    parts = product_part.split('-')
    
    # Look for "by-[instructor]" pattern
    instructor_parts = []
    found_by = False
    
    for i, part in enumerate(parts):
        if part == 'by' and i + 1 < len(parts):
            found_by = True
            # Take the next 1-3 parts as potential instructor name
            for j in range(i + 1, min(i + 4, len(parts))):
                if parts[j] and len(parts[j]) >= 2:
                    instructor_parts.append(parts[j].capitalize())
            break
    
    return instructor_parts


def match_series_to_existing_results(series_groups: Dict[str, List[str]], existing_results: Dict[str, str]) -> Dict[str, str]:
    """Match series to existing results using smarter URL analysis with better series matching.
    
    Args:
        series_groups: Dictionary mapping series keys to video file lists
        existing_results: Dictionary of existing URLs
        
    Returns:
        Dictionary mapping series keys to existing URLs
    """
    series_matches = {}
    used_urls = set()  # Track which URLs have been matched
    
    # First, extract instructor info from each existing URL
    url_instructors = {}
    for existing_url in existing_results.keys():
        instructors = extract_instructor_from_url(existing_url)
        if instructors:
            url_instructors[existing_url] = [name.lower() for name in instructors]
            logging.debug(f"URL {existing_url} -> instructors: {instructors}")
    
    # Create list of series with their info for better matching
    series_info = []
    for series_key, series_files in series_groups.items():
        representative_file = series_files[0]
        filename = Path(representative_file).name
        search_terms = parse_video_filename(filename)
        
        if search_terms.get('instructor'):
            series_instructor = [name.lower() for name in search_terms['instructor']]
            series_words = [word.lower() for word in search_terms.get('series', [])]
            series_info.append((series_key, series_instructor, series_words))
    
    # Calculate match scores for all series-URL combinations
    matches = []
    for series_key, series_instructor, series_words in series_info:
        for existing_url, url_instructor in url_instructors.items():
            # Calculate instructor match score
            instructor_score = 0
            for series_name in series_instructor:
                for url_name in url_instructor:
                    if len(series_name) >= 3 and series_name in url_name:
                        instructor_score += 2  # Exact match
                    elif len(url_name) >= 3 and url_name in series_name:
                        instructor_score += 2  # Exact match
            
            # Calculate series content match score (bonus for matching series words in URL)
            content_score = 0
            clean_url = clean_product_url(existing_url)
            for series_word in series_words:
                if len(series_word) >= 4 and series_word in clean_url:
                    content_score += 1
            
            total_score = instructor_score + content_score
            
            if total_score > 0:  # Must have at least instructor match
                matches.append((total_score, series_key, existing_url))
    
    # Sort matches by score (highest first) and assign URLs to series
    matches.sort(reverse=True)
    
    for score, series_key, existing_url in matches:
        # Skip if this URL is already used or this series is already matched
        if existing_url in used_urls or series_key in series_matches:
            continue
            
        series_matches[series_key] = existing_url
        used_urls.add(existing_url)
        logging.info(f"Series '{series_key}' matches existing result: {existing_url} (score: {score})")
    
    # Log series that couldn't be matched
    for series_key, _, _ in series_info:
        if series_key not in series_matches:
            logging.debug(f"No good match found for series '{series_key}'")
    
    return series_matches


def append_to_product_pages_file(video_paths: List[str], new_url: str) -> None:
    """Append a new product URL to product-pages.txt immediately.
    
    Args:
        video_paths: List of video file paths to determine output location
        new_url: New product URL to append
    """
    output_file = get_output_file_path(video_paths)
    
    try:
        # Append the new URL directly (no numbering)
        with open(output_file, 'a') as f:
            f.write(f"{new_url}\n")
        
        logging.info(f"Appended result to {output_file}: {new_url}")
        
    except Exception as e:
        logging.error(f"Failed to append to product-pages.txt: {e}")
        print(f"Error appending to product-pages.txt: {e}")


def write_product_pages_file(video_paths: List[str], results: Dict[str, Optional[str]]) -> None:
    """Write product page URLs to product-pages.txt in the video directory.
    
    Args:
        video_paths: List of video file paths
        results: Dictionary mapping filenames to product URLs
    """
    output_file = get_output_file_path(video_paths)
    
    try:
        with open(output_file, 'w') as f:
            for filename, url in results.items():
                if url:
                    # Write just the URL (no numbering or filename)
                    f.write(f"{url}\n")
        
        logging.info(f"Product pages written to: {output_file}")
        print(f"Product pages written to: {output_file}")
        
    except Exception as e:
        logging.error(f"Failed to write product-pages.txt: {e}")
        print(f"Error writing product-pages.txt: {e}")


def find_video_files(paths: List[str]) -> List[str]:
    """Find all video files from the given paths (files or directories), including subdirectories.
    
    Args:
        paths: List of file paths or directory paths
        
    Returns:
        List of video file paths
    """
    video_extensions = {'.mp4', '.avi', '.mkv', '.mov', '.wmv', '.flv', '.webm'}
    video_files = []
    
    for path_str in paths:
        path = Path(path_str)
        
        if path.is_file():
            # Single file - add if it's a video
            if path.suffix.lower() in video_extensions:
                video_files.append(str(path))
        elif path.is_dir():
            # Directory - find all video files recursively
            logging.info(f"Searching directory recursively: {path}")
            for video_file in path.rglob('*'):
                if video_file.is_file() and video_file.suffix.lower() in video_extensions:
                    video_files.append(str(video_file))
                    logging.debug(f"Found video file: {video_file}")
        else:
            logging.warning(f"Path not found: {path}")
    
    return sorted(video_files)


def group_videos_by_series(video_files: List[str]) -> Dict[str, List[str]]:
    """Group video files by series (instructor + series name).
    
    Args:
        video_files: List of video file paths
        
    Returns:
        Dictionary mapping series key to list of video files
    """
    series_groups = {}
    
    for filepath in video_files:
        filename = Path(filepath).name
        search_terms = parse_video_filename(filename)
        
        # Create series key from instructor + series (ignore episode numbers)
        instructor_key = ' '.join(search_terms['instructor']).lower()
        series_key = ' '.join(search_terms['series']).lower()
        
        # Combine instructor and series for unique key
        full_series_key = f"{instructor_key}|{series_key}"
        
        if full_series_key not in series_groups:
            series_groups[full_series_key] = []
        series_groups[full_series_key].append(filepath)
    
    return series_groups


def main():
    """Main function with CLI interface."""
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Find BJJ Fanatics product pages from video filenames or directories"
    )
    parser.add_argument(
        'paths', 
        nargs='+', 
        help='Video files or directories containing videos to process'
    )
    parser.add_argument(
        '-v', '--verbose', 
        action='store_true', 
        help='Enable verbose output'
    )
    parser.add_argument(
        '--force-refresh', 
        action='store_true', 
        help='Ignore existing results and search all series again'
    )
    
    args = parser.parse_args()
    
    # Setup logging
    setup_logging(args.verbose)
    
    # Find all video files from the given paths
    video_files = find_video_files(args.paths)
    
    if not video_files:
        print("No video files found in the specified paths")
        return 1
    
    print(f"Found {len(video_files)} video files to process")
    
    # Group videos by series to avoid duplicate searches
    series_groups = group_videos_by_series(video_files)
    print(f"Grouped into {len(series_groups)} unique series")
    
    # Load existing results (unless force refresh is enabled)
    existing_results = {}
    series_matches = {}
    if not args.force_refresh:
        existing_results = load_existing_results(video_files)
        if existing_results:
            series_matches = match_series_to_existing_results(series_groups, existing_results)
            print(f"Found {len(series_matches)} series already cached in product-pages.txt")
    
    # Determine which series need to be searched
    series_to_search = {k: v for k, v in series_groups.items() if k not in series_matches}
    
    if not series_to_search:
        print("All series already have results! Use --force-refresh to search again.")
        # Still show the cached results
        results = {}
        for series_key, series_files in series_groups.items():
            cached_url = series_matches.get(series_key)
            for filepath in series_files:
                results[filepath] = cached_url
                if cached_url:
                    print(f"  {Path(filepath).name} -> {cached_url} (cached)")
                else:
                    print(f"  {Path(filepath).name} -> NOT FOUND")
    else:
        print(f"Need to search {len(series_to_search)} new series")
        
        # Process series that need searching
        results = {}
        series_results = {}
        
        # First, apply cached results
        for series_key, cached_url in series_matches.items():
            series_results[series_key] = cached_url
            for filepath in series_groups[series_key]:
                results[filepath] = cached_url
                if cached_url:
                    print(f"  {Path(filepath).name} -> {cached_url} (cached)")
        
        # Then search for new series
        for i, (series_key, series_files) in enumerate(series_to_search.items()):
            # Use the first file in the series for searching
            representative_file = series_files[0]
            filename = Path(representative_file).name
            
            print(f"Processing series {i+1}/{len(series_to_search)}: {filename} (represents {len(series_files)} videos)")
            
            product_url = find_product_page(filename)
            series_results[series_key] = product_url
            
            # Apply the result to all videos in this series
            for filepath in series_files:
                results[filepath] = product_url
                if product_url:
                    print(f"  {Path(filepath).name} -> {product_url}")
                else:
                    print(f"  {Path(filepath).name} -> NOT FOUND")
            
            # Append result immediately to file (crash protection)
            if product_url:
                append_to_product_pages_file(video_files, product_url)
                print(f"  ✓ Saved to product-pages.txt")
            
            # Add delay between series searches (except for last item)
            if i < len(series_to_search) - 1:
                delay = random.uniform(10, 15)  # Random delay between 10-15 seconds
                print(f"Waiting {delay:.1f} seconds before next series... ({i+1}/{len(series_to_search)} series completed)")
                time.sleep(delay)
    
    # Final summary
    found_count = sum(1 for url in results.values() if url)
    total_count = len(results)
    series_found = sum(1 for series_key in series_groups.keys() if series_key in series_matches or (series_key in series_to_search and results.get(series_groups[series_key][0])))
    total_series = len(series_groups)
    cached_count = len(series_matches)
    new_count = series_found - cached_count
    
    print(f"\nSummary: {series_found}/{total_series} series found ({cached_count} cached, {new_count} new), {found_count}/{total_count} total videos matched")
    
    return 0 if found_count > 0 else 1


if __name__ == "__main__":
    sys.exit(main())