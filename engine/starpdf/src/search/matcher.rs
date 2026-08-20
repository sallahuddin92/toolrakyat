use crate::search::result::{SearchBoundingBox, SearchOptions, SearchResult};
use crate::text::span::PageText;

pub struct TextMatcher;

impl TextMatcher {
    /// Searches for a query string across spans in a `PageText`, preserving exact multi-box geometry.
    pub fn search_page(
        page_text: &PageText,
        query: &str,
        options: &SearchOptions,
    ) -> Vec<SearchResult> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() || page_text.spans.is_empty() {
            return Vec::new();
        }

        // 1. Build concatenated string with mapping from char index to (span_index, char_in_span)
        let mut full_text = String::new();
        let mut char_map: Vec<(usize, usize)> = Vec::new();

        for (span_idx, span) in page_text.spans.iter().enumerate() {
            if !full_text.is_empty() && !full_text.ends_with(' ') {
                full_text.push(' ');
                // Sentinel for inter-span separator space
                char_map.push((span_idx, usize::MAX));
            }

            for (char_idx, ch) in span.text.chars().enumerate() {
                full_text.push(ch);
                char_map.push((span_idx, char_idx));
            }
        }

        // 2. Perform search
        let hay = if options.case_sensitive {
            full_text.clone()
        } else {
            full_text.to_lowercase()
        };

        let needle = if options.case_sensitive {
            trimmed_query.to_string()
        } else {
            trimmed_query.to_lowercase()
        };

        let mut results = Vec::new();
        let mut search_start = 0;

        while let Some(found_pos) = hay[search_start..].find(&needle) {
            let match_start_char = search_start + found_pos;
            let match_end_char = match_start_char + needle.len();
            search_start = match_start_char + 1;

            if match_end_char > char_map.len() {
                break;
            }

            // Extract the original matched text substring
            let matched_text = full_text[match_start_char..match_end_char].to_string();

            // Find participating span range
            let mut span_sub_slices: Vec<(usize, usize, usize)> = Vec::new(); // (span_idx, min_char, max_char)

            for char_idx in match_start_char..match_end_char {
                let (span_idx, char_in_span) = char_map[char_idx];
                if char_in_span == usize::MAX {
                    continue; // Skip separator space
                }

                if let Some(last) = span_sub_slices.last_mut() {
                    if last.0 == span_idx {
                        last.2 = last.2.max(char_in_span + 1);
                        continue;
                    }
                }
                span_sub_slices.push((span_idx, char_in_span, char_in_span + 1));
            }

            if span_sub_slices.is_empty() {
                continue;
            }

            let start_span_index = span_sub_slices.first().map_or(0, |s| s.0);
            let end_span_index = span_sub_slices.last().map_or(0, |s| s.0);

            // Generate bounding boxes for each span segment
            let mut boxes = Vec::with_capacity(span_sub_slices.len());
            let mut min_confidence: f64 = 1.0;

            for &(span_idx, min_c, max_c) in &span_sub_slices {
                let span = &page_text.spans[span_idx];
                min_confidence = min_confidence.min(span.confidence);

                let total_chars = span.text.chars().count().max(1);
                let frac_start = min_c as f64 / total_chars as f64;
                let frac_len = (max_c - min_c) as f64 / total_chars as f64;

                let sub_width = span.width * frac_len;
                let rad = span.rotation.to_radians();
                let delta_x = (span.width * frac_start) * rad.cos();
                let delta_y = (span.width * frac_start) * rad.sin();

                boxes.push(SearchBoundingBox::new(
                    page_text.page_index,
                    span.x + delta_x,
                    span.y + delta_y,
                    sub_width,
                    span.height,
                    span.rotation,
                ));
            }

            results.push(SearchResult {
                page_index: page_text.page_index,
                matched_text,
                start_span_index,
                end_span_index,
                boxes,
                confidence: min_confidence,
            });
        }

        results
    }
}
