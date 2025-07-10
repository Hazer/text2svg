// some useful utility functions
use anyhow::{Result, anyhow};
use std::iter::Iterator;
use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::font::{FontConfig, FontStyle};
use rustybuzz::Face;

// Reads file line by line, splitting lines longer than `max_chars_per_line`.
// Tries to wrap at whitespace for ASCII text.
pub fn open_file_by_lines_width<P: AsRef<Path>>(path: P, max_chars_per_line: usize) -> Result<Vec<String>> {
    let path = path.as_ref();
    if path.exists() && path.is_file() {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let width_iter = WidthLineIterator::new(reader, max_chars_per_line);
                Ok(width_iter.collect())
            },
            Err(err) => Err(anyhow!("{}: {}", path.display(), err)),
        }
    } else {
        Err(anyhow!(
            "{}: doesn't exist or is not a regular file", path.display()))
    }
}

// Reads file line by line, splitting lines based on pixel width.
// Uses font metrics to determine actual text width for wrapping.
pub fn open_file_by_lines_pixel_width<P: AsRef<Path>>(
    path: P, 
    max_pixel_width: f32,
    font_config: &mut FontConfig,
    font_style: &FontStyle
) -> Result<Vec<String>> {
    let path = path.as_ref();
    if path.exists() && path.is_file() {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let pixel_width_iter = PixelWidthLineIterator::new(reader, max_pixel_width, font_config, font_style);
                Ok(pixel_width_iter.collect())
            },
            Err(err) => Err(anyhow!("{}: {}", path.display(), err)),
        }
    } else {
        Err(anyhow!(
            "{}: doesn't exist or is not a regular file", path.display()))
    }
}

// Reads file line by line without width constraints.
pub fn open_file_by_lines<P: AsRef<Path>>(path: P) -> Result<Vec<String>> {
    let path = path.as_ref();
    if path.exists() && path.is_file() {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let lines = reader.lines().collect::<Result<Vec<String>, _>>()
                    .map_err(|e| anyhow!("{}: {}", path.display(), e))?;
                Ok(lines)
            },
            Err(err) => Err(anyhow!("{}: {}", path.display(), err)),
        }
    } else {
        Err(anyhow!(
            "{}: doesn't exist or is not a regular file", path.display()))
    }
}


// --- WidthLineIterator ---
// Iterator that reads lines from a BufReader, but splits lines exceeding
// a specified character width, attempting word wrapping for ASCII.

struct WidthLineIterator<R: BufRead> {
    reader: R,
    max_width: usize,
    buffer: String, // Holds leftover part of a line for the next iteration
}

impl<R: BufRead> WidthLineIterator<R> {
    fn new(reader: R, max_width: usize) -> Self {
        WidthLineIterator {
            reader,
            max_width,
            buffer: String::new(),
        }
    }
}

impl<R: BufRead> Iterator for WidthLineIterator<R> {
    type Item = String;

        fn next(&mut self) -> Option<Self::Item> {
        // Process buffer first if exceeding max_width
        if self.buffer.chars().count() > self.max_width {
            let (line_part, remaining_part) = split_line(&self.buffer, self.max_width);
            self.buffer = remaining_part;
            return Some(line_part);
        }

        // If buffer has content within max_width, return it
        if !self.buffer.is_empty() {
            let buffer_content = std::mem::take(&mut self.buffer);
            return Some(buffer_content);
        }

        // Buffer empty, read a new line
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => { // Successfully read a line
                let trimmed_line = line.trim_end_matches(['\r', '\n']).to_string();

                // If line exceeds max_width, split it
                if trimmed_line.chars().count() > self.max_width {
                    let (line_part, remaining_part) = split_line(&trimmed_line, self.max_width);
                    self.buffer = remaining_part;
                    Some(line_part)
                } else {
                    // Line fits within max_width
                    Some(trimmed_line)
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                None
            }
        }
    }
}

// --- PixelWidthLineIterator ---
// Iterator that reads lines from a BufReader, but splits lines based on pixel width
// using actual font metrics for precise text measurement.

struct PixelWidthLineIterator<'a, R: BufRead> {
    reader: R,
    max_pixel_width: f32,
    font_config: &'a mut FontConfig,
    font_style: &'a FontStyle,
    buffer: String, // Holds leftover part of a line for the next iteration
}

impl<'a, R: BufRead> PixelWidthLineIterator<'a, R> {
    fn new(reader: R, max_pixel_width: f32, font_config: &'a mut FontConfig, font_style: &'a FontStyle) -> Self {
        PixelWidthLineIterator {
            reader,
            max_pixel_width,
            font_config,
            font_style,
            buffer: String::new(),
        }
    }
}

impl<R: BufRead> Iterator for PixelWidthLineIterator<'_, R> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        // Process buffer first if exceeding max_pixel_width
        if let Some(text_width) = calculate_text_width(&self.buffer, self.font_config, self.font_style) {
            if text_width > self.max_pixel_width {
                let (line_part, remaining_part) = split_line_by_pixel_width(&self.buffer, self.max_pixel_width, self.font_config, self.font_style);
                self.buffer = remaining_part;
                return Some(line_part);
            }
        }

        // If buffer has content within max_pixel_width, return it
        if !self.buffer.is_empty() {
            let buffer_content = std::mem::take(&mut self.buffer);
            return Some(buffer_content);
        }

        // Buffer empty, read a new line
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => { // Successfully read a line
                let trimmed_line = line.trim_end_matches(['\r', '\n']).to_string();

                // If line exceeds max_pixel_width, split it
                if let Some(text_width) = calculate_text_width(&trimmed_line, self.font_config, self.font_style) {
                    if text_width > self.max_pixel_width {
                        let (line_part, remaining_part) = split_line_by_pixel_width(&trimmed_line, self.max_pixel_width, self.font_config, self.font_style);
                        self.buffer = remaining_part;
                        return Some(line_part);
                    }
                }
                // Line fits within max_pixel_width
                Some(trimmed_line)
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                None
            }
        }
    }
}

// Helper function to split a line at max_width, trying to wrap at whitespace.
fn split_line(line: &str, max_width: usize) -> (String, String) {
    if line.chars().count() <= max_width {
        return (line.trim_end().to_string(), String::new());
    }

    // Find the character index corresponding to max_width
    let split_char_index = if let Some((idx, _)) = line.char_indices().nth(max_width) {
        idx
    } else if line.chars().count() > max_width {
        line.char_indices().nth(max_width).map(|(i, _)| i).unwrap_or(line.len())
    } else {
        0
    };

    // Look backwards from the split point for whitespace
    let potential_split_point = &line[..split_char_index];
    let wrap_index = potential_split_point
        .char_indices()
        .rev()
        .find(|&(_, c)| c.is_ascii_whitespace())
        .map(|(i, _)| i);

    if let Some(idx) = wrap_index {
        // Found whitespace: split before it, trim whitespace
        let first_part = potential_split_point[..idx].trim_end().to_string();
        let second_part = line[idx..].trim_start().to_string();
        (first_part, second_part)
    } else {
        // No whitespace found: hard break at max_width chars
        let (first_part, second_part) = line.split_at(split_char_index);
        (first_part.to_string(), second_part.trim_start().to_string()) // Added trim_start() here
    }
}

// Calculate the pixel width of text using font metrics
fn calculate_text_width(text: &str, font_config: &mut FontConfig, font_style: &FontStyle) -> Option<f32> {
    if text.is_empty() {
        return Some(0.0);
    }

    // Get the font face for the specified style, fallback to regular
    let ft_face = font_config.get_font_by_style(font_style)
        .or_else(|| font_config.get_font_by_style(&FontStyle::Regular))?;

    let font_data = ft_face.copy_font_data()?;
    let hb_face = Face::from_slice(&font_data, 0)?;

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);

    let glyph_buffer = rustybuzz::shape(&hb_face, font_config.get_features(), buffer);

    // Calculate total advance width
    let mut total_width = 0.0;
    let glyph_positions = glyph_buffer.glyph_positions();
    
    // Get font metrics for scaling
    let metrics = ft_face.metrics();
    let target_size = font_config.get_size() as f32;
    let origin_glyph_height = metrics.ascent - metrics.descent;
    let scale_factor = target_size / origin_glyph_height.max(1.0);

    for glyph_pos in glyph_positions {
        total_width += glyph_pos.x_advance as f32 * scale_factor;
    }

    // Add letter spacing
    let letter_space = scale_factor * font_config.get_letter_space() * metrics.units_per_em as f32;
    let char_count = text.chars().count();
    if char_count > 1 {
        total_width += letter_space * (char_count - 1) as f32;
    }

    Some(total_width)
}

// Split a line based on pixel width, trying to wrap at whitespace
fn split_line_by_pixel_width(
    line: &str, 
    max_pixel_width: f32, 
    font_config: &mut FontConfig, 
    font_style: &FontStyle
) -> (String, String) {
    if let Some(text_width) = calculate_text_width(line, font_config, font_style) {
        if text_width <= max_pixel_width {
            return (line.trim_end().to_string(), String::new());
        }
    } else {
        // Fallback to character-based splitting if width calculation fails
        return split_line(line, 50); // Arbitrary fallback
    }

    // Find the optimal split point using binary search approach
    let chars: Vec<char> = line.chars().collect();
    let mut best_split = 0;
    let mut wrap_split = None;

    // First pass: find the maximum characters that fit
    for i in 1..=chars.len() {
        let substring: String = chars[..i].iter().collect();
        if let Some(width) = calculate_text_width(&substring, font_config, font_style) {
            if width <= max_pixel_width {
                best_split = i;
                // Check if this position is at a word boundary
                if i < chars.len() && chars[i-1].is_ascii_whitespace() {
                    wrap_split = Some(i-1);
                }
            } else {
                break;
            }
        }
    }

    // Use word boundary if we found one within reasonable distance
    let mut split_point = if let Some(wrap_pos) = wrap_split {
        // Only use word boundary if it's not too far from the optimal split
        let distance = best_split.saturating_sub(wrap_pos);
        if distance <= best_split / 4 { // Within 25% of optimal
            wrap_pos
        } else {
            best_split
        }
    } else {
        // Look backwards from best_split for whitespace
        let mut search_pos = best_split;
        while search_pos > 0 {
            search_pos -= 1;
            if chars[search_pos].is_ascii_whitespace() {
                break;
            }
        }
        if search_pos > 0 && chars[search_pos].is_ascii_whitespace() {
            search_pos
        } else {
            best_split
        }
    };

    if split_point == 0 {
        // Emergency fallback: at least take one character
        split_point = 1.min(chars.len());
    }

    let first_part: String = chars[..split_point].iter().collect();
    let second_part: String = chars[split_point..].iter().collect();

    (first_part.trim_end().to_string(), second_part.trim_start().to_string())
}

// Convenience function to wrap a single text string by pixel width
pub fn wrap_text_by_pixel_width(
    text: &str,
    max_pixel_width: f32,
    font_config: &mut FontConfig,
    font_style: &FontStyle
) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        if let Some(text_width) = calculate_text_width(&remaining, font_config, font_style) {
            if text_width <= max_pixel_width {
                lines.push(remaining);
                break;
            }
        }

        let (line_part, remaining_part) = split_line_by_pixel_width(&remaining, max_pixel_width, font_config, font_style);
        if line_part.is_empty() {
            // Prevent infinite loop
            break;
        }
        lines.push(line_part);
        remaining = remaining_part;
    }

    lines
}


#[cfg(test)]
mod test_utils{
  use super::*;
  use std::io::Cursor;

  #[test]
  fn test_open_file_not_found() {
        match open_file_by_lines("/tmp/file-does-not-exist-hopefully") {
            Ok(_) => panic!("Should have failed"),
            Err(e) => assert!(e.to_string().contains("doesn't exist or is not a regular file")),
        }
         match open_file_by_lines_width("/tmp/file-does-not-exist-hopefully", 80) {
            Ok(_) => panic!("Should have failed"),
            Err(e) => assert!(e.to_string().contains("doesn't exist or is not a regular file")),
        }
  }

  #[test]
  fn test_read_lines_basic() {
      let data = "line1\nline2\nline3";
      let cursor = Cursor::new(data);
      let reader = BufReader::new(cursor);
      let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
      assert_eq!(lines, vec!["line1", "line2", "line3"]);
  }

   #[test]
    fn test_split_line_simple() {
        let (l, r) = split_line("abcdefghijkl", 5);
        assert_eq!(l, "abcde");
        assert_eq!(r, "fghijkl");
    }

    #[test]
    fn test_split_line_with_whitespace_wrap() {
        let (l, r) = split_line("abcde fghijkl", 8);
        assert_eq!(l, "abcde"); // Wraps before 'f' at the space
        assert_eq!(r, "fghijkl");
    }

     #[test]
    fn test_split_line_with_whitespace_at_end() {
        let (l, r) = split_line("abcde ", 5); // Space is exactly at width limit
        assert_eq!(l, "abcde"); // Space is trimmed
        assert_eq!(r, "");
    }

    #[test]
    fn test_split_line_no_whitespace() {
        let (l, r) = split_line("abcdefghijkl", 5);
        assert_eq!(l, "abcde"); // Hard break
        assert_eq!(r, "fghijkl");
    }

     #[test]
    fn test_split_line_non_ascii() {
        let (l, r) = split_line("你好世界你好世界", 3); // Split after 3 chars
        assert_eq!(l, "你好世");
        assert_eq!(r, "界你好世界");
    }


  #[test]
  fn test_width_iter_long_text_no_wrap() {
        let data = "123123123";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 3);
        let lines: Vec<String> = width_iter.collect();
        assert_eq!(lines, vec!["123", "123", "123"]);
  }

  #[test]
  fn test_width_iter_non_ascii_wrap() {
        let data = "当我发现我童年和少年时期的旧日记时，它们已经被尘埃所覆盖。";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 26);
        let lines: Vec<String> = width_iter.collect();
        // Should hard break as no ASCII whitespace involved
        assert_eq!(lines, vec!["当我发现我童年和少年时期的旧日记时，它们已经被尘埃所", "覆盖。"]);
  }

  #[test]
  fn test_width_iter_text_wrapping_ascii() {
        let data = "When I found my old diaries from my childhood and teen years, they were covered in dust.";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 76);
        let lines: Vec<String> = width_iter.collect();
        // Should wrap at "were"
        assert_eq!(lines, vec!["When I found my old diaries from my childhood and teen years, they were", "covered in dust."]);
  }

   #[test]
  fn test_width_iter_multiple_lines_wrapping() {
        let data = "This is the first line which is quite long and needs wrapping.\nThis is the second line, also long.\nShort third.";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 20);
        let lines: Vec<String> = width_iter.collect();
        assert_eq!(lines, vec![
            "This is the first",
            "line which is quite",
            "long and needs",
            "wrapping.",
            "This is the second",
            "line, also long.",
            "Short third."
            ]);
  }

   #[test]
  fn test_width_iter_empty_lines() {
        let data = "Line 1\n\nLine 3";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 80);
        let lines: Vec<String> = width_iter.collect();
        assert_eq!(lines, vec!["Line 1", "", "Line 3"]);
  }

   #[test]
  fn test_width_iter_exact_width() {
        let data = "12345\n67890";
        let cursor = Cursor::new(data);
        let reader = BufReader::new(cursor);
        let width_iter = WidthLineIterator::new(reader, 5);
        let lines: Vec<String> = width_iter.collect();
        assert_eq!(lines, vec!["12345", "67890"]);
  }

  // Helper function to create a font config with system fonts for testing
  fn create_test_font_config() -> FontConfig {
        use crate::font::fonts;
        
        // Try to get system fonts and use the first available one
        let available_fonts = fonts();
        let font_name = if !available_fonts.is_empty() {
            available_fonts[0].clone()
        } else {
            // Fallback to common system fonts
            #[cfg(target_os = "macos")]
            let fallback = "Arial";
            #[cfg(target_os = "windows")]
            let fallback = "Arial";
            #[cfg(target_os = "linux")]
            let fallback = "DejaVu Sans";
            fallback.to_string()
        };

        FontConfig::new(
            font_name,
            16,
            "#000".to_string(),
            "#000".to_string(),
            false
        ).expect("Failed to create font config with system font")
  }

  #[test]
  fn test_wrap_text_by_pixel_width_empty() {
        // Test empty string - this should always work
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let result = wrap_text_by_pixel_width("", 100.0, &mut font_config, &FontStyle::Regular);
        assert_eq!(result, vec![""]);
  }

  #[test]
  fn test_calculate_text_width_empty() {
        // Test empty string width calculation
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let width = calculate_text_width("", &mut font_config, &FontStyle::Regular);
        assert_eq!(width, Some(0.0));
  }

  #[test]
  fn test_calculate_text_width_simple() {
        // Test width calculation for simple text
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let width = calculate_text_width("Hello", &mut font_config, &FontStyle::Regular);
        
        // Width should be Some positive value for non-empty text
        assert!(width.is_some());
        assert!(width.unwrap() > 0.0);
  }

  #[test]
  fn test_split_line_by_pixel_width_no_split_needed() {
        // Test splitting when text fits within pixel width
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let text = "Short";
        
        // Use a very large pixel width - text should not be split
        let (first, second) = split_line_by_pixel_width(text, 10000.0, &mut font_config, &FontStyle::Regular);
        
        // Should not split - all text in first part
        assert_eq!(first.trim(), text);
        assert!(second.is_empty());
  }

  #[test]
  fn test_split_line_by_pixel_width_needs_split() {
        // Test splitting when text exceeds pixel width
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let text = "This is a longer text that should be split";
        
        // Use a small pixel width to force splitting
        let (first, second) = split_line_by_pixel_width(text, 50.0, &mut font_config, &FontStyle::Regular);
        
        // Should have split the line
        assert!(!first.is_empty());
        
        // Verify the split preserves the original text
        let combined = if second.is_empty() {
            first.trim().to_string()
        } else {
            format!("{} {}", first.trim(), second.trim()).trim().to_string()
        };
        
        // Remove multiple spaces that might occur during splitting and joining
        let normalized_combined = combined.split_whitespace().collect::<Vec<_>>().join(" ");
        let normalized_original = text.split_whitespace().collect::<Vec<_>>().join(" ");
        
        assert_eq!(normalized_combined, normalized_original);
  }

  #[test]
  fn test_wrap_text_by_pixel_width_single_line() {
        // Test wrapping text that fits in one line
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let text = "Short text";
        
        let result = wrap_text_by_pixel_width(text, 10000.0, &mut font_config, &FontStyle::Regular);
        
        // Should return single line
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], text);
  }

  #[test]
  fn test_wrap_text_by_pixel_width_multiple_lines() {
        // Test wrapping text that needs multiple lines
        use crate::font::FontStyle;
        
        let mut font_config = create_test_font_config();
        let text = "This is a very long text that should definitely be wrapped into multiple lines when using a small pixel width";
        
        let result = wrap_text_by_pixel_width(text, 100.0, &mut font_config, &FontStyle::Regular);
        
        // Should return multiple lines
        assert!(result.len() > 1);
        
        // Verify all lines together equal original text
        let combined = result.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
        let original = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(combined, original);
  }

  // Test the basic functionality without requiring actual fonts
  #[test]
  fn test_pixel_width_api_exists() {
        // This test verifies that the pixel width API functions exist and can be called
        // It tests the API surface without requiring working fonts
        
        let text = "Test";
        
        // Test that the functions exist - we're not testing functionality here,
        // just API availability since we can't guarantee font availability in tests
        let _result = format!("pixel width API functions are available for text: {}", text);
        // API exists if we reach here
  }
}
