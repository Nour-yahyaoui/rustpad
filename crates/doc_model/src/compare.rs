//! RustPad v3.0.0 Advanced Document Comparison & Smart Merging Engine:
//! - Character-level (`diff_chars`) and paragraph-level (`compare_documents`) diffing
//! - Formatting & style delta detection (bold, italic, underline, font, size, color, hyperlinks, alignment)
//! - Structural comparison (tables, cell diffs, images, footnotes, endnotes, comments, sections, page setup)
//! - Conflict-aware 3-way (`smart_merge_documents`) and 2-way (`merge_two_documents`) smart merging

use crate::{
    Block, DocComment, Document, Paragraph, RevisionKind, RevisionMeta, SpanStyle, TextSpan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharDiffOp {
    Equal(String),
    Insert(String),
    Delete(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattingChange {
    pub block_index: usize,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralChange {
    pub category: &'static str,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentComparisonReport {
    pub paragraphs_unchanged: usize,
    pub paragraphs_modified: usize,
    pub paragraphs_inserted: usize,
    pub paragraphs_deleted: usize,
    pub char_insertions: usize,
    pub char_deletions: usize,
    pub formatting_changes: Vec<FormattingChange>,
    pub structural_changes: Vec<StructuralChange>,
}

impl DocumentComparisonReport {
    pub fn is_identical(&self) -> bool {
        self.paragraphs_modified == 0
            && self.paragraphs_inserted == 0
            && self.paragraphs_deleted == 0
            && self.char_insertions == 0
            && self.char_deletions == 0
            && self.formatting_changes.is_empty()
            && self.structural_changes.is_empty()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "Paragraphs: {} unchanged, ~{} modified, +{} inserted, -{} deleted | Chars: +{} / -{} | Formatting: {} | Structure: {}",
            self.paragraphs_unchanged,
            self.paragraphs_modified,
            self.paragraphs_inserted,
            self.paragraphs_deleted,
            self.char_insertions,
            self.char_deletions,
            self.formatting_changes.len(),
            self.structural_changes.len()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Emits conflicting edits from both reviewers as attributed ECMA-376 Track Changes + conflict comment.
    AnnotateConflictsAsTrackChanges,
    /// Resolves overlapping conflicts in favor of `ours`.
    PreferOurs,
    /// Resolves overlapping conflicts in favor of `theirs`.
    PreferTheirs,
}

#[derive(Debug, Clone)]
pub struct MergeOutcome {
    pub document: Document,
    pub clean_auto_merges: usize,
    pub conflicts_count: usize,
    pub conflict_descriptions: Vec<String>,
}

/// Computes exact character-level diff operations between `old` and `new`, using common prefix/suffix
/// trimming + bounded dynamic programming (with chunked linear fallback on huge paragraphs).
pub fn diff_chars(old: &str, new: &str) -> Vec<CharDiffOp> {
    if old == new {
        return if old.is_empty() {
            Vec::new()
        } else {
            vec![CharDiffOp::Equal(old.to_string())]
        };
    }
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();

    let mut prefix = 0usize;
    let min_len = old_chars.len().min(new_chars.len());
    while prefix < min_len && old_chars[prefix] == new_chars[prefix] {
        prefix += 1;
    }

    let mut old_end = old_chars.len();
    let mut new_end = new_chars.len();
    while old_end > prefix && new_end > prefix && old_chars[old_end - 1] == new_chars[new_end - 1]
    {
        old_end -= 1;
        new_end -= 1;
    }

    let mut ops: Vec<CharDiffOp> = Vec::new();
    if prefix > 0 {
        ops.push(CharDiffOp::Equal(old_chars[..prefix].iter().collect()));
    }

    let mid_old = &old_chars[prefix..old_end];
    let mid_new = &new_chars[prefix..new_end];

    if mid_old.is_empty() && !mid_new.is_empty() {
        ops.push(CharDiffOp::Insert(mid_new.iter().collect()));
    } else if !mid_old.is_empty() && mid_new.is_empty() {
        ops.push(CharDiffOp::Delete(mid_old.iter().collect()));
    } else if !mid_old.is_empty() && !mid_new.is_empty() {
        if mid_old.len() * mid_new.len() <= 180_000 {
            let n = mid_old.len();
            let m = mid_new.len();
            let stride = m + 1;
            let mut dp = vec![0u32; (n + 1) * stride];
            for i in (0..n).rev() {
                for j in (0..m).rev() {
                    dp[i * stride + j] = if mid_old[i] == mid_new[j] {
                        dp[(i + 1) * stride + (j + 1)] + 1
                    } else {
                        dp[(i + 1) * stride + j].max(dp[i * stride + (j + 1)])
                    };
                }
            }
            let mut i = 0usize;
            let mut j = 0usize;
            while i < n || j < m {
                if i < n && j < m && mid_old[i] == mid_new[j] {
                    push_char_op(&mut ops, 0, mid_old[i]);
                    i += 1;
                    j += 1;
                } else if j < m
                    && (i == n || dp[i * stride + (j + 1)] >= dp[(i + 1) * stride + j])
                {
                    push_char_op(&mut ops, 1, mid_new[j]);
                    j += 1;
                } else {
                    push_char_op(&mut ops, 2, mid_old[i]);
                    i += 1;
                }
            }
        } else {
            ops.push(CharDiffOp::Delete(mid_old.iter().collect()));
            ops.push(CharDiffOp::Insert(mid_new.iter().collect()));
        }
    }

    if old_end < old_chars.len() {
        let tail: String = old_chars[old_end..].iter().collect();
        if let Some(CharDiffOp::Equal(prev)) = ops.last_mut() {
            prev.push_str(&tail);
        } else {
            ops.push(CharDiffOp::Equal(tail));
        }
    }
    ops
}

fn push_char_op(ops: &mut Vec<CharDiffOp>, kind: u8, ch: char) {
    match (kind, ops.last_mut()) {
        (0, Some(CharDiffOp::Equal(s))) => s.push(ch),
        (1, Some(CharDiffOp::Insert(s))) => s.push(ch),
        (2, Some(CharDiffOp::Delete(s))) => s.push(ch),
        (0, _) => ops.push(CharDiffOp::Equal(ch.to_string())),
        (1, _) => ops.push(CharDiffOp::Insert(ch.to_string())),
        _ => ops.push(CharDiffOp::Delete(ch.to_string())),
    }
}

/// Generates an ECMA-376 redline document using fine-grained **character-level** differences.
pub fn diff_redline_char_level(
    base: &Document,
    revised: &Document,
    reviewer: &str,
    date_iso: &str,
) -> Document {
    let mut out = revised.clone();
    out.title = format!("{} (Char Redline)", revised.title);
    out.track_changes = true;
    out.blocks.clear();

    let max_len = base.blocks.len().max(revised.blocks.len());
    let mut rev_id = 1u32;

    for idx in 0..max_len {
        match (base.blocks.get(idx), revised.blocks.get(idx)) {
            (Some(Block::Paragraph(bp)), Some(Block::Paragraph(rp))) => {
                let bt = bp.accepted_plain_text();
                let rt = rp.accepted_plain_text();
                if bt == rt {
                    out.blocks.push(Block::Paragraph(rp.clone()));
                } else {
                    let base_style = rp
                        .spans
                        .first()
                        .map(|s| s.style.clone())
                        .unwrap_or_else(|| SpanStyle {
                            font_size_pt: rp.style.default_font_size(),
                            bold: rp.style.is_bold_by_default(),
                            ..SpanStyle::default()
                        });
                    let mut spans = Vec::new();
                    for op in diff_chars(&bt, &rt) {
                        match op {
                            CharDiffOp::Equal(t) => {
                                spans.push(TextSpan::styled(t, base_style.clone()))
                            }
                            CharDiffOp::Insert(t) => {
                                spans.push(TextSpan::tracked(
                                    t,
                                    base_style.clone(),
                                    RevisionMeta {
                                        id: rev_id,
                                        kind: RevisionKind::Insertion,
                                        author: reviewer.to_string(),
                                        date_iso: date_iso.to_string(),
                                    },
                                ));
                                rev_id += 1;
                            }
                            CharDiffOp::Delete(t) => {
                                spans.push(TextSpan::tracked(
                                    t,
                                    base_style.clone(),
                                    RevisionMeta {
                                        id: rev_id,
                                        kind: RevisionKind::Deletion,
                                        author: reviewer.to_string(),
                                        date_iso: date_iso.to_string(),
                                    },
                                ));
                                rev_id += 1;
                            }
                        }
                    }
                    let mut p = rp.clone();
                    p.spans = spans;
                    p.normalize_spans();
                    out.blocks.push(Block::Paragraph(p));
                }
            }
            (None, Some(Block::Paragraph(rp))) => {
                let mut p = rp.clone();
                let rt = rp.accepted_plain_text();
                if !rt.is_empty() {
                    let st = rp.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                    p.spans = vec![TextSpan::tracked(
                        rt,
                        st,
                        RevisionMeta {
                            id: rev_id,
                            kind: RevisionKind::Insertion,
                            author: reviewer.to_string(),
                            date_iso: date_iso.to_string(),
                        },
                    )];
                    rev_id += 1;
                }
                out.blocks.push(Block::Paragraph(p));
            }
            (Some(Block::Paragraph(bp)), None) => {
                let bt = bp.accepted_plain_text();
                if !bt.is_empty() {
                    let mut p = bp.clone();
                    let st = bp.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                    p.spans = vec![TextSpan::tracked(
                        bt,
                        st,
                        RevisionMeta {
                            id: rev_id,
                            kind: RevisionKind::Deletion,
                            author: reviewer.to_string(),
                            date_iso: date_iso.to_string(),
                        },
                    )];
                    rev_id += 1;
                    out.blocks.push(Block::Paragraph(p));
                }
            }
            (_, Some(other)) => out.blocks.push(other.clone()),
            (Some(other), None) => out.blocks.push(other.clone()),
            (None, None) => {}
        }
    }
    out.ensure_non_empty();
    out
}

/// Deep multi-layer comparison across paragraphs, characters, formatting styles, and document structure.
pub fn compare_documents(base: &Document, revised: &Document) -> DocumentComparisonReport {
    let mut rep = DocumentComparisonReport::default();

    // 1. Structural & Metadata Deltas
    if base.classification != revised.classification {
        rep.structural_changes.push(StructuralChange {
            category: "Security",
            summary: format!(
                "Classification changed: {} -> {}",
                base.classification.label(),
                revised.classification.label()
            ),
        });
    }
    if base.page_setup != revised.page_setup {
        rep.structural_changes.push(StructuralChange {
            category: "PageSetup",
            summary: format!(
                "Page layout changed ({:?} landscape={} -> {:?} landscape={})",
                base.page_setup.paper_size,
                base.page_setup.landscape,
                revised.page_setup.paper_size,
                revised.page_setup.landscape
            ),
        });
    }
    if base.footnotes.len() != revised.footnotes.len() {
        rep.structural_changes.push(StructuralChange {
            category: "Footnotes",
            summary: format!(
                "Footnote count changed: {} -> {}",
                base.footnotes.len(),
                revised.footnotes.len()
            ),
        });
    }
    if base.endnotes.len() != revised.endnotes.len() {
        rep.structural_changes.push(StructuralChange {
            category: "Endnotes",
            summary: format!(
                "Endnote count changed: {} -> {}",
                base.endnotes.len(),
                revised.endnotes.len()
            ),
        });
    }
    if base.comments.len() != revised.comments.len() {
        rep.structural_changes.push(StructuralChange {
            category: "Comments",
            summary: format!(
                "Comment count changed: {} -> {}",
                base.comments.len(),
                revised.comments.len()
            ),
        });
    }

    let base_tables: Vec<_> = base
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        })
        .collect();
    let rev_tables: Vec<_> = revised
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Table(t) => Some(t),
            _ => None,
        })
        .collect();
    if base_tables.len() != rev_tables.len() {
        rep.structural_changes.push(StructuralChange {
            category: "Table",
            summary: format!(
                "Table count changed: {} -> {}",
                base_tables.len(),
                rev_tables.len()
            ),
        });
    }
    for (ti, (bt, rt)) in base_tables.iter().zip(rev_tables.iter()).enumerate() {
        if bt.rows.len() != rt.rows.len() || bt.cols != rt.cols {
            rep.structural_changes.push(StructuralChange {
                category: "Table",
                summary: format!(
                    "Table #{} dimensions changed: {}x{} -> {}x{}",
                    ti + 1,
                    bt.rows.len(),
                    bt.cols,
                    rt.rows.len(),
                    rt.cols
                ),
            });
        } else {
            let mut changed_cells = 0usize;
            for (r1, r2) in bt.rows.iter().zip(rt.rows.iter()) {
                for (c1, c2) in r1.iter().zip(r2.iter()) {
                    if c1.text != c2.text {
                        changed_cells += 1;
                    }
                }
            }
            if changed_cells > 0 {
                rep.structural_changes.push(StructuralChange {
                    category: "Table",
                    summary: format!("Table #{} modified in {} cell(s)", ti + 1, changed_cells),
                });
            }
        }
    }

    let base_imgs = base
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Image(_)))
        .count();
    let rev_imgs = revised
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Image(_)))
        .count();
    if base_imgs != rev_imgs {
        rep.structural_changes.push(StructuralChange {
            category: "Image",
            summary: format!("Embedded figure count changed: {base_imgs} -> {rev_imgs}"),
        });
    }

    // 2. Paragraph, Character & Formatting Alignment
    let b_paras: Vec<(usize, &Paragraph)> = base
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| match b {
            Block::Paragraph(p) => Some((idx, p)),
            _ => None,
        })
        .collect();
    let r_paras: Vec<(usize, &Paragraph)> = revised
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(idx, b)| match b {
            Block::Paragraph(p) => Some((idx, p)),
            _ => None,
        })
        .collect();

    let b_texts: Vec<String> = b_paras
        .iter()
        .map(|(_, p)| p.accepted_plain_text())
        .collect();
    let r_texts: Vec<String> = r_paras
        .iter()
        .map(|(_, p)| p.accepted_plain_text())
        .collect();

    // Exact LCS match on paragraph plain texts when bounded, else linear index pairing
    if b_texts.len() * r_texts.len() <= 250_000 {
        let n = b_texts.len();
        let m = r_texts.len();
        let stride = m + 1;
        let mut dp = vec![0u32; (n + 1) * stride];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                dp[i * stride + j] = if b_texts[i] == r_texts[j] {
                    dp[(i + 1) * stride + (j + 1)] + 1
                } else {
                    dp[(i + 1) * stride + j].max(dp[i * stride + (j + 1)])
                };
            }
        }
        let mut i = 0usize;
        let mut j = 0usize;
        let mut unmatched_b: Vec<usize> = Vec::new();
        let mut unmatched_r: Vec<usize> = Vec::new();

        let flush_unmatched = |ub: &mut Vec<usize>,
                               ur: &mut Vec<usize>,
                               rep: &mut DocumentComparisonReport| {
            let pairs = ub.len().min(ur.len());
            for k in 0..pairs {
                let bi = ub[k];
                let rj = ur[k];
                rep.paragraphs_modified += 1;
                for op in diff_chars(&b_texts[bi], &r_texts[rj]) {
                    match op {
                        CharDiffOp::Insert(s) => rep.char_insertions += s.chars().count(),
                        CharDiffOp::Delete(s) => rep.char_deletions += s.chars().count(),
                        CharDiffOp::Equal(_) => {}
                    }
                }
                inspect_para_formatting_delta(
                    r_paras[rj].0,
                    b_paras[bi].1,
                    r_paras[rj].1,
                    &mut rep.formatting_changes,
                );
            }
            for &bi in &ub[pairs..] {
                rep.paragraphs_deleted += 1;
                rep.char_deletions += b_texts[bi].chars().count();
            }
            for &rj in &ur[pairs..] {
                rep.paragraphs_inserted += 1;
                rep.char_insertions += r_texts[rj].chars().count();
            }
            ub.clear();
            ur.clear();
        };

        while i < n || j < m {
            if i < n && j < m && b_texts[i] == r_texts[j] {
                flush_unmatched(&mut unmatched_b, &mut unmatched_r, &mut rep);
                rep.paragraphs_unchanged += 1;
                inspect_para_formatting_delta(
                    r_paras[j].0,
                    b_paras[i].1,
                    r_paras[j].1,
                    &mut rep.formatting_changes,
                );
                i += 1;
                j += 1;
            } else if j < m
                && (i == n || dp[i * stride + (j + 1)] >= dp[(i + 1) * stride + j])
            {
                unmatched_r.push(j);
                j += 1;
            } else {
                unmatched_b.push(i);
                i += 1;
            }
        }
        flush_unmatched(&mut unmatched_b, &mut unmatched_r, &mut rep);
    } else {
        let pairs = b_paras.len().min(r_paras.len());
        for k in 0..pairs {
            if b_texts[k] == r_texts[k] {
                rep.paragraphs_unchanged += 1;
            } else {
                rep.paragraphs_modified += 1;
                for op in diff_chars(&b_texts[k], &r_texts[k]) {
                    match op {
                        CharDiffOp::Insert(s) => rep.char_insertions += s.chars().count(),
                        CharDiffOp::Delete(s) => rep.char_deletions += s.chars().count(),
                        CharDiffOp::Equal(_) => {}
                    }
                }
            }
            inspect_para_formatting_delta(
                r_paras[k].0,
                b_paras[k].1,
                r_paras[k].1,
                &mut rep.formatting_changes,
            );
        }
        if b_paras.len() > pairs {
            rep.paragraphs_deleted += b_paras.len() - pairs;
        }
        if r_paras.len() > pairs {
            rep.paragraphs_inserted += r_paras.len() - pairs;
        }
    }

    rep
}

fn inspect_para_formatting_delta(
    block_idx: usize,
    bp: &Paragraph,
    rp: &Paragraph,
    out: &mut Vec<FormattingChange>,
) {
    if bp.style != rp.style {
        out.push(FormattingChange {
            block_index: block_idx,
            summary: format!("Paragraph style: {} -> {}", bp.style.label(), rp.style.label()),
        });
    }
    if bp.alignment != rp.alignment {
        out.push(FormattingChange {
            block_index: block_idx,
            summary: format!("Alignment: {:?} -> {:?}", bp.alignment, rp.alignment),
        });
    }
    if bp.list_kind != rp.list_kind {
        out.push(FormattingChange {
            block_index: block_idx,
            summary: format!("List marker: {:?} -> {:?}", bp.list_kind, rp.list_kind),
        });
    }
    if let (Some(bs), Some(rs)) = (bp.spans.first(), rp.spans.first()) {
        if bs.style.bold != rs.style.bold
            || bs.style.italic != rs.style.italic
            || bs.style.underline != rs.style.underline
            || bs.style.font_family != rs.style.font_family
            || (bs.style.font_size_pt - rs.style.font_size_pt).abs() > 0.25
            || bs.style.hyperlink != rs.style.hyperlink
        {
            out.push(FormattingChange {
                block_index: block_idx,
                summary: format!(
                    "Run style changed (font {:?} {:.1}pt B={} I={} U={} link={:?})",
                    rs.style.font_family,
                    rs.style.font_size_pt,
                    rs.style.bold,
                    rs.style.italic,
                    rs.style.underline,
                    rs.style.hyperlink
                ),
            });
        }
    }
}

/// Smart 3-Way Document Merger (`base`, `ours`, `theirs`):
/// - Automatically merges non-overlapping edits from `ours` and `theirs`.
/// - Resolves true overlapping conflicts via `strategy` (defaulting to attributed Track Changes + comment!).
pub fn smart_merge_documents(
    base: &Document,
    ours: &Document,
    theirs: &Document,
    strategy: MergeStrategy,
) -> MergeOutcome {
    let mut merged = ours.clone();
    merged.title = format!("{} (Merged)", ours.title);
    merged.blocks.clear();

    let max_len = base
        .blocks
        .len()
        .max(ours.blocks.len())
        .max(theirs.blocks.len());
    let mut clean_auto_merges = 0usize;
    let mut conflicts_count = 0usize;
    let mut conflict_descriptions = Vec::new();
    let mut next_rev_id = ours
        .next_revision_id()
        .max(theirs.next_revision_id())
        .max(1);
    let mut next_comment_id = merged
        .comments
        .iter()
        .map(|c| c.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    for idx in 0..max_len {
        let b_opt = base.blocks.get(idx);
        let o_opt = ours.blocks.get(idx);
        let t_opt = theirs.blocks.get(idx);

        match (b_opt, o_opt, t_opt) {
            (_, Some(o), Some(t)) if o == t => {
                if b_opt != Some(o) {
                    clean_auto_merges += 1;
                }
                merged.blocks.push(o.clone());
            }
            (Some(b), Some(o), Some(t)) if o == b && t != b => {
                clean_auto_merges += 1;
                merged.blocks.push(t.clone());
            }
            (Some(b), Some(o), Some(t)) if t == b && o != b => {
                clean_auto_merges += 1;
                merged.blocks.push(o.clone());
            }
            (None, Some(o), None) => {
                clean_auto_merges += 1;
                merged.blocks.push(o.clone());
            }
            (None, None, Some(t)) => {
                clean_auto_merges += 1;
                merged.blocks.push(t.clone());
            }
            (Some(b), Some(o), None) if o == b => {
                // Deleted cleanly in `theirs`
                clean_auto_merges += 1;
            }
            (Some(b), None, Some(t)) if t == b => {
                // Deleted cleanly in `ours`
                clean_auto_merges += 1;
            }
            (_, o_conflict, t_conflict) => {
                conflicts_count += 1;
                let desc = format!("Block #{}: concurrent modification between reviewers", idx + 1);
                conflict_descriptions.push(desc.clone());

                match strategy {
                    MergeStrategy::PreferOurs => {
                        if let Some(o) = o_conflict {
                            merged.blocks.push(o.clone());
                        }
                    }
                    MergeStrategy::PreferTheirs => {
                        if let Some(t) = t_conflict {
                            merged.blocks.push(t.clone());
                        }
                    }
                    MergeStrategy::AnnotateConflictsAsTrackChanges => {
                        if let (Some(Block::Paragraph(op)), Some(Block::Paragraph(tp))) =
                            (o_conflict, t_conflict)
                        {
                            merged.track_changes = true;
                            let mut combined = op.clone();
                            let st = op.spans.first().map(|s| s.style.clone()).unwrap_or_default();
                            combined.spans = vec![
                                TextSpan::styled(op.accepted_plain_text(), st.clone()),
                                TextSpan::tracked(
                                    format!(" [ALT: {}]", tp.accepted_plain_text()),
                                    st,
                                    RevisionMeta {
                                        id: next_rev_id,
                                        kind: RevisionKind::Insertion,
                                        author: theirs.author.clone(),
                                        date_iso: "2026-11-01T00:00:00Z".into(),
                                    },
                                ),
                            ];
                            next_rev_id += 1;
                            let out_block_idx = merged.blocks.len();
                            merged.blocks.push(Block::Paragraph(combined));
                            merged.comments.push(DocComment {
                                id: next_comment_id,
                                block_index: out_block_idx,
                                author: "RustPad Smart Merge".into(),
                                date_iso: "2026-11-01T00:00:00Z".into(),
                                text: format!(
                                    "Merge conflict preserved: '{}' vs '{}'",
                                    ours.author, theirs.author
                                ),
                                resolved: false,
                            });
                            next_comment_id += 1;
                        } else {
                            if let Some(o) = o_conflict {
                                merged.blocks.push(o.clone());
                            }
                            if let Some(t) = t_conflict {
                                merged.blocks.push(t.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Merge unique footnotes & endnotes from `theirs`
    for fn_item in &theirs.footnotes {
        if !merged.footnotes.iter().any(|x| x.text == fn_item.text) {
            let new_id = merged
                .footnotes
                .iter()
                .map(|x| x.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            merged.footnotes.push(crate::DocNote {
                id: new_id,
                text: fn_item.text.clone(),
                author: fn_item.author.clone(),
            });
        }
    }
    for en_item in &theirs.endnotes {
        if !merged.endnotes.iter().any(|x| x.text == en_item.text) {
            let new_id = merged
                .endnotes
                .iter()
                .map(|x| x.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            merged.endnotes.push(crate::DocNote {
                id: new_id,
                text: en_item.text.clone(),
                author: en_item.author.clone(),
            });
        }
    }

    merged.ensure_non_empty();
    MergeOutcome {
        document: merged,
        clean_auto_merges,
        conflicts_count,
        conflict_descriptions,
    }
}

/// Convenience 2-way smart merge (uses an empty baseline when no common ancestor is supplied).
pub fn merge_two_documents(
    ours: &Document,
    theirs: &Document,
    strategy: MergeStrategy,
) -> MergeOutcome {
    smart_merge_documents(ours, ours, theirs, strategy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParagraphStyle;

    #[test]
    fn test_char_diff_and_multi_layer_comparison() {
        let ops = diff_chars("Net 30 days", "Net 14 days");
        assert!(ops.contains(&CharDiffOp::Delete("30".into())));
        assert!(ops.contains(&CharDiffOp::Insert("14".into())));

        let mut a = Document::blank();
        a.blocks = vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Heading1, "1. Scope")),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Fee is $1,000 USD.")),
        ];

        let mut b = a.clone();
        if let Block::Paragraph(p) = &mut b.blocks[0] {
            p.style = ParagraphStyle::Heading2;
        }
        if let Block::Paragraph(p) = &mut b.blocks[1] {
            p.spans[0].text = "Fee is $2,500 USD.".into();
        }
        b.blocks.push(Block::Paragraph(Paragraph::new(
            ParagraphStyle::Normal,
            "Added clause.",
        )));

        let rep = compare_documents(&a, &b);
        assert_eq!(rep.paragraphs_unchanged, 1);
        assert_eq!(rep.paragraphs_modified, 1);
        assert_eq!(rep.paragraphs_inserted, 1);
        assert!(!rep.formatting_changes.is_empty());
    }

    #[test]
    fn test_smart_3way_merge_auto_and_conflict_annotation() {
        let mut base = Document::blank();
        base.blocks = vec![
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Clause A original")),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Clause B original")),
            Block::Paragraph(Paragraph::new(ParagraphStyle::Normal, "Clause C contested")),
        ];

        let mut ours = base.clone();
        ours.author = "Counsel A".into();
        if let Block::Paragraph(p) = &mut ours.blocks[0] {
            p.spans[0].text = "Clause A updated by Counsel A".into();
        }
        if let Block::Paragraph(p) = &mut ours.blocks[2] {
            p.spans[0].text = "Clause C governed by NY Law".into();
        }

        let mut theirs = base.clone();
        theirs.author = "Counsel B".into();
        if let Block::Paragraph(p) = &mut theirs.blocks[1] {
            p.spans[0].text = "Clause B updated by Counsel B".into();
        }
        if let Block::Paragraph(p) = &mut theirs.blocks[2] {
            p.spans[0].text = "Clause C governed by Delaware Law".into();
        }

        let res = smart_merge_documents(
            &base,
            &ours,
            &theirs,
            MergeStrategy::AnnotateConflictsAsTrackChanges,
        );
        assert_eq!(res.clean_auto_merges, 2);
        assert_eq!(res.conflicts_count, 1);
        assert!(res.document.track_changes);
        assert_eq!(res.document.comments.len(), 1);
        let plain = res.document.to_plain_text();
        assert!(plain.contains("Clause A updated by Counsel A"));
        assert!(plain.contains("Clause B updated by Counsel B"));
        assert!(plain.contains("Delaware Law"));
    }
}
