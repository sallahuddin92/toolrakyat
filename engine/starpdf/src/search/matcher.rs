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

        // 1. Build character array with mapping from char index to (span_index, char_in_span)
        let mut char_vec: Vec<char> = Vec::new();
        let mut char_map: Vec<(usize, usize)> = Vec::new();

        for (span_idx, span) in page_text.spans.iter().enumerate() {
            let joins_previous_fragment = span_idx
                .checked_sub(1)
                .and_then(|index| page_text.spans.get(index))
                .is_some_and(|previous| Self::overlapping_line_fragment(previous, span));
            if !char_vec.is_empty()
                && char_vec.last().is_some_and(|ch| !ch.is_whitespace())
                && !span.text.starts_with(char::is_whitespace)
                && !joins_previous_fragment
            {
                char_vec.push(' ');
                // Sentinel for inter-span separator space
                char_map.push((span_idx, usize::MAX));
            }

            for (char_idx, ch) in span.text.chars().enumerate() {
                char_vec.push(ch);
                char_map.push((span_idx, char_idx));
            }
        }

        let full_text: String = char_vec.iter().collect();

        let query_chars: Vec<char> = if options.case_sensitive {
            trimmed_query.chars().collect()
        } else {
            trimmed_query.to_lowercase().chars().collect()
        };

        let hay_chars: Vec<char> = if options.case_sensitive {
            char_vec.clone()
        } else {
            full_text.to_lowercase().chars().collect()
        };

        if query_chars.is_empty() || hay_chars.len() < query_chars.len() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let query_len = query_chars.len();

        for i in 0..=(hay_chars.len() - query_len) {
            if hay_chars[i..i + query_len] == query_chars[..] {
                let match_start_char = i;
                let match_end_char = i + query_len;

                let matched_text: String =
                    char_vec[match_start_char..match_end_char].iter().collect();

                // Find participating span range
                let mut span_sub_slices: Vec<(usize, usize, usize)> = Vec::new(); // (span_idx, min_char, max_char)

                for &item in char_map.iter().take(match_end_char).skip(match_start_char) {
                    let (span_idx, char_in_span) = item;
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
        }

        results
    }

    fn overlapping_line_fragment(
        previous: &crate::text::span::TextSpan,
        current: &crate::text::span::TextSpan,
    ) -> bool {
        if previous.text.ends_with(char::is_whitespace)
            || current.text.starts_with(char::is_whitespace)
            || (previous.rotation - current.rotation).abs() > 0.01
        {
            return false;
        }
        let radians = previous.rotation.to_radians();
        let previous_end_x = previous.x + previous.width * radians.cos();
        let previous_end_y = previous.y + previous.width * radians.sin();
        let advance_gap = (current.x - previous_end_x) * radians.cos()
            + (current.y - previous_end_y) * radians.sin();
        let cross_gap = -(current.x - previous_end_x) * radians.sin()
            + (current.y - previous_end_y) * radians.cos();
        advance_gap <= previous.font_size.max(current.font_size).max(1.0) * 0.1
            && cross_gap.abs() <= previous.height.max(current.height).max(4.0) * 0.8
    }
}
