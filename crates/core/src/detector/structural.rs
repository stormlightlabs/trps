//! Structural detectors for trope signals that are not literal phrase matches.

use super::Finding;

/// Finds structural trope signals in text.
pub fn scan_structural(text: &str) -> Vec<Finding> {
    let sentences = sentence_spans(text);
    let paragraphs = paragraph_spans(text);

    let mut findings = Vec::new();
    findings.extend(scan_anaphora(text, &sentences));
    findings.extend(scan_tricolon(text, &sentences));
    findings.extend(scan_short_punchy_fragments(text, &sentences));
    findings.extend(scan_listicle_in_trench_coat(text, &paragraphs));
    findings.extend(scan_fractal_summaries(text, &paragraphs));
    findings.extend(scan_historical_analogy_stacking(text, &sentences));
    findings
}

fn scan_anaphora(text: &str, sentences: &[super::Span]) -> Vec<Finding> {
    let starts: Vec<_> = sentences
        .iter()
        .filter_map(|sentence| sentence_start_key(text, *sentence).map(|key| (*sentence, key)))
        .collect();

    starts
        .windows(3)
        .filter(|window| window[0].1 == window[1].1 && window[1].1 == window[2].1)
        .map(|window| {
            Finding::structural(
                ("sentence_structure.anaphora_abuse", "Anaphora Abuse"),
                text,
                super::Span(window[0].0.start(), window[2].0.end()),
            )
        })
        .collect()
}

fn scan_tricolon(text: &str, sentences: &[super::Span]) -> Vec<Finding> {
    sentences
        .iter()
        .filter(|sentence| {
            let value = &text[sentence.start()..sentence.end()];
            let separators = value.matches(',').count() + value.matches(';').count();
            separators >= 2 && repeated_clause_starts(value) >= 2
        })
        .map(|sentence| {
            Finding::structural(
                ("sentence_structure.tricolon_abuse", "Tricolon Abuse"),
                text,
                super::Span(sentence.start(), sentence.end()),
            )
        })
        .collect()
}

fn scan_short_punchy_fragments(text: &str, sentences: &[super::Span]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut run_start = None;
    let mut run_end = 0;
    let mut run_len = 0;

    for sentence in sentences {
        if word_count(&text[sentence.start()..sentence.end()]) <= 4 {
            run_start.get_or_insert(sentence.start());
            run_end = sentence.end();
            run_len += 1;
        } else {
            if run_len >= 3 {
                findings.push(Finding::structural(
                    (
                        "paragraph_structure.short_punchy_fragments",
                        "Short Punchy Fragments",
                    ),
                    text,
                    super::Span(run_start.unwrap(), run_end),
                ));
            }
            run_start = None;
            run_end = 0;
            run_len = 0;
        }
    }

    if run_len >= 3 {
        findings.push(Finding::structural(
            (
                "paragraph_structure.short_punchy_fragments",
                "Short Punchy Fragments",
            ),
            text,
            super::Span(run_start.unwrap(), run_end),
        ));
    }

    findings
}

fn scan_listicle_in_trench_coat(text: &str, paragraphs: &[super::Span]) -> Vec<Finding> {
    let mut ordinal_hits = Vec::new();

    for paragraph in paragraphs {
        if paragraph_starts_with_ordinal(&text[paragraph.start()..paragraph.end()]) {
            ordinal_hits.push(*paragraph);
        }
    }

    ordinal_hits
        .windows(3)
        .map(|window| {
            Finding::structural(
                (
                    "paragraph_structure.listicle_in_trench_coat",
                    "Listicle in a Trench Coat",
                ),
                text,
                super::Span(window[0].start(), window[2].end()),
            )
        })
        .collect()
}

fn scan_fractal_summaries(text: &str, paragraphs: &[super::Span]) -> Vec<Finding> {
    let mut hits = Vec::new();

    for paragraph in paragraphs {
        let value = text[paragraph.start()..paragraph.end()].trim_start();
        if starts_with_any_ci(
            value,
            &[
                "in this section",
                "as we've seen",
                "as we have seen",
                "in summary",
                "to sum up",
                "in conclusion",
            ],
        ) {
            hits.push(*paragraph);
        }
    }

    if hits.len() < 3 {
        return Vec::new();
    }

    vec![Finding::structural(
        ("composition.fractal_summaries", "Fractal Summaries"),
        text,
        super::Span(hits[0].start(), hits[hits.len() - 1].end()),
    )]
}

fn scan_historical_analogy_stacking(text: &str, sentences: &[super::Span]) -> Vec<Finding> {
    sentences
        .windows(3)
        .filter(|window| {
            window
                .iter()
                .all(|sentence| has_analogy_marker(&text[sentence.start()..sentence.end()]))
        })
        .map(|window| {
            Finding::structural(
                (
                    "composition.historical_analogy_stacking",
                    "Historical Analogy Stacking",
                ),
                text,
                super::Span(window[0].start(), window[2].end()),
            )
        })
        .collect()
}

fn sentence_spans(text: &str) -> Vec<super::Span> {
    let mut spans = Vec::new();
    let mut start = 0;

    for (index, character) in text.char_indices() {
        if matches!(character, '.' | '!' | '?') {
            push_trimmed_span(text, &mut spans, start, index + character.len_utf8());
            start = index + character.len_utf8();
        }
    }

    push_trimmed_span(text, &mut spans, start, text.len());
    spans
}

fn paragraph_spans(text: &str) -> Vec<super::Span> {
    let mut spans = Vec::new();
    let mut start = 0;

    for (index, _) in text.match_indices("\n\n") {
        push_trimmed_span(text, &mut spans, start, index);
        start = index + 2;
    }

    push_trimmed_span(text, &mut spans, start, text.len());
    spans
}

fn push_trimmed_span(text: &str, spans: &mut Vec<super::Span>, start: usize, end: usize) {
    let value = &text[start..end];
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return;
    }

    let leading = value.len() - value.trim_start().len();
    let trailing = value.len() - value.trim_end().len();
    spans.push(super::Span(start + leading, end - trailing));
}

fn sentence_start_key(text: &str, sentence: super::Span) -> Option<String> {
    let words = words(&text[sentence.start()..sentence.end()]);

    if words.is_empty() {
        None
    } else {
        Some(words.into_iter().take(2).collect::<Vec<_>>().join(" "))
    }
}

fn repeated_clause_starts(sentence: &str) -> usize {
    let starts: Vec<_> = sentence
        .split([',', ';'])
        .filter_map(|clause| words(clause).into_iter().next())
        .collect();

    starts
        .windows(2)
        .filter(|window| window[0] == window[1])
        .count()
}

fn paragraph_starts_with_ordinal(paragraph: &str) -> bool {
    starts_with_any_ci(
        paragraph.trim_start(),
        &[
            "the first",
            "first,",
            "first ",
            "the second",
            "second,",
            "second ",
            "the third",
            "third,",
        ],
    )
}

fn has_analogy_marker(sentence: &str) -> bool {
    let lower = sentence.to_ascii_lowercase();
    let examples = [
        "apple", "facebook", "stripe", "aws", "spotify", "uber", "airbnb", "shopify", "discord",
        "web", "mobile", "social", "cloud",
    ];

    examples.iter().any(|example| lower.contains(example))
}

fn starts_with_any_ci(value: &str, prefixes: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    prefixes.iter().any(|prefix| lower.starts_with(prefix))
}

fn word_count(value: &str) -> usize {
    words(value).len()
}

fn words(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_anaphora_abuse() {
        let findings = scan_structural(
            "They assume users pay. They assume builders arrive. They assume markets form.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "sentence_structure.anaphora_abuse")
        );
    }

    #[test]
    fn detects_tricolon_abuse() {
        let findings = scan_structural(
            "Products impress people, products empower teams, products create worlds.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "sentence_structure.tricolon_abuse")
        );
    }

    #[test]
    fn detects_short_punchy_fragments() {
        let findings = scan_structural("He published this. Openly. In a book. As a priest.");

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "paragraph_structure.short_punchy_fragments")
        );
    }

    #[test]
    fn detects_listicle_in_trench_coat() {
        let findings = scan_structural(
            "The first wall is access.\n\nThe second wall is pricing.\n\nThe third wall is trust.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "paragraph_structure.listicle_in_trench_coat")
        );
    }

    #[test]
    fn detects_fractal_summaries() {
        let findings = scan_structural(
            "In this section, we examine access.\n\nAs we've seen, access matters.\n\nIn summary, access wins.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.fractal_summaries")
        );
    }

    #[test]
    fn detects_historical_analogy_stacking() {
        let findings = scan_structural(
            "Apple did not build Uber. Facebook did not build Spotify. AWS did not build Airbnb.",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule_id == "composition.historical_analogy_stacking")
        );
    }
}
