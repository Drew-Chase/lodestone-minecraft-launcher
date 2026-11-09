//! A three-way line merge of a text file: the pristine copy the installed release shipped (`base`), the file as the user has it
//! (`yours`), and the file the release being installed ships (`theirs`). Changes that touch different lines merge on their own.
//! Where both sides changed the same lines differently, the merge reports a conflict, which the user resolves hunk by hunk, the
//! way `git merge` leaves it for them. Line endings pass through byte for byte.

use std::ops::Range;

use similar::{Algorithm, DiffOp, capture_diff_slices};

/// Which side a conflict resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The file as the user has it.
    Yours,
    /// The file the release being installed ships.
    Theirs,
}

/// One piece of a merged file: lines that need no decision, or a place the user decides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece<'a> {
    /// Lines both sides agree on, or that only one side changed. They are written as they are.
    Lines(Vec<&'a str>),
    /// A place both sides changed differently. `yours` is the file's lines there as the user has them, and `theirs` as the release
    /// ships them.
    Conflict { yours: Vec<&'a str>, theirs: Vec<&'a str> },
}

/// A run of one side's lines that replaces the base lines from `start` to `end`. An insertion has `start == end`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change<'a> {
    start: usize,
    end: usize,
    with: Vec<&'a str>,
}

/// Merges `yours` and `theirs`, both changed from `base`, in file order.
pub(crate) fn merge<'a>(base: &'a str, yours: &'a str, theirs: &'a str) -> Vec<Piece<'a>> {
    let (base, yours, theirs) = (lines(base), lines(yours), lines(theirs));
    merge_lines(&base, &yours, &theirs, false)
}

/// Compares `yours` with `theirs` when there is no base to tell which side changed a line: every place they differ is a
/// conflict for the user to decide.
pub(crate) fn compare<'a>(yours: &'a str, theirs: &'a str) -> Vec<Piece<'a>> {
    let (yours, theirs) = (lines(yours), lines(theirs));
    merge_lines(&theirs, &yours, &theirs, true)
}

/// The text the pieces make, with conflict number `index` (counted from the top of the file) resolved to `choose(index)`.
pub(crate) fn render(pieces: &[Piece<'_>], choose: impl Fn(usize) -> Side) -> String {
    let mut text = String::new();
    let mut conflicts = 0;
    for piece in pieces {
        match piece {
            Piece::Lines(lines) => lines.iter().for_each(|line| text.push_str(line)),
            Piece::Conflict { yours, theirs } => {
                let chosen = match choose(conflicts) {
                    Side::Yours => yours,
                    Side::Theirs => theirs,
                };
                chosen.iter().for_each(|line| text.push_str(line));
                conflicts += 1;
            }
        }
    }
    text
}

/// The number of conflicts in `pieces`.
#[cfg(test)]
pub(crate) fn conflicts(pieces: &[Piece<'_>]) -> usize {
    pieces.iter().filter(|piece| matches!(piece, Piece::Conflict { .. })).count()
}

/// `text` split into lines, each keeping its line ending.
fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// The changes that turn `base` into `side`, in base order.
fn changes<'a>(base: &[&'a str], side: &[&'a str]) -> Vec<Change<'a>> {
    capture_diff_slices(Algorithm::Myers, base, side)
        .into_iter()
        .filter_map(|op| match op {
            DiffOp::Equal { .. } => None,
            DiffOp::Delete { old_index, old_len, .. } => Some(Change {
                start: old_index,
                end: old_index + old_len,
                with: Vec::new(),
            }),
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => Some(Change {
                start: old_index,
                end: old_index,
                with: side[new_index..new_index + new_len].to_vec(),
            }),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => Some(Change {
                start: old_index,
                end: old_index + old_len,
                with: side[new_index..new_index + new_len].to_vec(),
            }),
        })
        .collect()
}

/// The lines of `base[range]` with the changes of `owner` in `group` applied.
fn applied<'a>(base: &[&'a str], range: Range<usize>, group: &[(Side, Change<'a>)], owner: Side) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut at = range.start;
    for (side, change) in group {
        if *side != owner {
            continue;
        }
        out.extend_from_slice(&base[at..change.start]);
        out.extend_from_slice(&change.with);
        at = change.end;
    }
    out.extend_from_slice(&base[at..range.end]);
    out
}

/// Merges `yours` and `theirs` against `base`. With `two_way`, `base` stands in for the release and any change of `yours` is a
/// conflict, since nothing says the user made it.
fn merge_lines<'a>(base: &[&'a str], yours: &[&'a str], theirs: &[&'a str], two_way: bool) -> Vec<Piece<'a>> {
    let mut all: Vec<(Side, Change<'a>)> = changes(base, yours)
        .into_iter()
        .map(|change| (Side::Yours, change))
        .chain(changes(base, theirs).into_iter().map(|change| (Side::Theirs, change)))
        .collect();
    all.sort_by_key(|(_, change)| (change.start, change.end));

    let mut pieces = Vec::new();
    let mut at = 0;
    let mut index = 0;
    while index < all.len() {
        let start = all[index].1.start;
        let mut end = all[index].1.end;
        let mut next = index + 1;
        // A change that touches the region's end belongs to it, so adjacent changes from both sides are one conflict to git's rule.
        while next < all.len() && all[next].1.start <= end {
            end = end.max(all[next].1.end);
            next += 1;
        }
        if at < start {
            pieces.push(Piece::Lines(base[at..start].to_vec()));
        }
        let group = &all[index..next];
        let yours_region = applied(base, start..end, group, Side::Yours);
        let theirs_region = applied(base, start..end, group, Side::Theirs);
        let yours_changed = group.iter().any(|(side, _)| *side == Side::Yours);
        let theirs_changed = group.iter().any(|(side, _)| *side == Side::Theirs);
        let piece = if two_way && yours_changed {
            Piece::Conflict {
                yours: yours_region,
                theirs: base[start..end].to_vec(),
            }
        } else if !theirs_changed || (yours_changed && yours_region == theirs_region) {
            Piece::Lines(yours_region)
        } else if !yours_changed {
            Piece::Lines(theirs_region)
        } else {
            Piece::Conflict {
                yours: yours_region,
                theirs: theirs_region,
            }
        };
        pieces.push(piece);
        at = end;
        index = next;
    }
    if at < base.len() {
        pieces.push(Piece::Lines(base[at..].to_vec()));
    }
    pieces
}

#[cfg(test)]
#[path = "../../../../tests/app/install/update/merge.rs"]
mod tests;
