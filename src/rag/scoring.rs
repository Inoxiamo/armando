use super::{HybridAggregate, RetrievedDocument, ScoredChunk};
use std::cmp::Ordering;
use std::collections::HashMap;

pub(super) fn normalize_vector_score(score: f32) -> f32 {
    ((score + 1.0) / 2.0).clamp(0.0, 1.0)
}

pub(super) fn normalize_keyword_score(raw_score: f32, min_raw: f32, max_raw: f32) -> f32 {
    if (max_raw - min_raw).abs() < f32::EPSILON {
        1.0
    } else {
        ((max_raw - raw_score) / (max_raw - min_raw)).clamp(0.0, 1.0)
    }
}

pub(super) fn normalize_keyword_scores(candidates: &[ScoredChunk]) -> HashMap<(String, i64), f32> {
    if candidates.is_empty() {
        return HashMap::new();
    }

    let min_raw = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::INFINITY, f32::min);
    let max_raw = candidates
        .iter()
        .map(|candidate| candidate.score)
        .fold(f32::NEG_INFINITY, f32::max);

    candidates
        .iter()
        .map(|candidate| {
            (
                (candidate.file_path.clone(), candidate.chunk_index),
                normalize_keyword_score(candidate.score, min_raw, max_raw),
            )
        })
        .collect()
}

pub(super) fn merge_vector_candidates(
    merged: &mut HashMap<(String, i64), HybridAggregate>,
    vector_candidates: Vec<ScoredChunk>,
) {
    for candidate in vector_candidates {
        let key = (candidate.file_path.clone(), candidate.chunk_index);
        let normalized = normalize_vector_score(candidate.score);
        merged
            .entry(key)
            .and_modify(|entry| {
                entry.vector_score = Some(normalized);
            })
            .or_insert_with(|| HybridAggregate {
                file_path: candidate.file_path,
                chunk_text: candidate.chunk_text,
                vector_score: Some(normalized),
                keyword_score: None,
            });
    }
}

pub(super) fn merge_keyword_candidates(
    merged: &mut HashMap<(String, i64), HybridAggregate>,
    keyword_candidates: Vec<ScoredChunk>,
) {
    let keyword_scores = normalize_keyword_scores(&keyword_candidates);
    for candidate in keyword_candidates {
        let key = (candidate.file_path.clone(), candidate.chunk_index);
        let normalized = keyword_scores.get(&key).copied().unwrap_or(1.0);
        merged
            .entry(key)
            .and_modify(|entry| {
                entry.keyword_score = Some(normalized);
            })
            .or_insert_with(|| HybridAggregate {
                file_path: candidate.file_path,
                chunk_text: candidate.chunk_text,
                vector_score: None,
                keyword_score: Some(normalized),
            });
    }
}

pub(super) fn finalize_hybrid_results(
    merged: HashMap<(String, i64), HybridAggregate>,
    top_n: usize,
) -> Vec<RetrievedDocument> {
    let mut scored = merged
        .into_values()
        .map(|entry| {
            let mut total = 0.0f32;
            let mut count = 0.0f32;
            if let Some(score) = entry.vector_score {
                total += score;
                count += 1.0;
            }
            if let Some(score) = entry.keyword_score {
                total += score;
                count += 1.0;
            }
            RetrievedDocument {
                file_path: entry.file_path,
                chunk_text: entry.chunk_text,
                score: if count > 0.0 { total / count } else { 0.0 },
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    scored.truncate(top_n);
    scored
}
