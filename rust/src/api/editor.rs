use flutter_rust_bridge::frb;
use zed_sum_tree::{Item, SumTree, Summary};
use crate::api::rope::RopeBridge;
use std::collections::HashSet;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

// -------------------------------------------------------------------------
// 1. ZED-SUM-TREE for Cursor Position and Fast Layout Tracking
// -------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct LineBlock {
    pub len_chars: usize,
    pub height: f32,
    pub is_folded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineSummary {
    pub len_chars: usize,
    pub height: f32,
    pub lines: usize,
}

impl Summary for LineSummary {
    type Context<'a> = ();
    
    fn zero(_cx: ()) -> Self {
        Self::default()
    }
    
    fn add_summary(&mut self, other: &Self, _: ()) {
        self.len_chars += other.len_chars;
        self.height += other.height;
        self.lines += other.lines;
    }
}

impl Item for LineBlock {
    type Summary = LineSummary;
    fn summary(&self, _: ()) -> Self::Summary {
        LineSummary {
            len_chars: self.len_chars,
            height: if self.is_folded { 0.0 } else { self.height },
            lines: 1,
        }
    }
}

#[frb(opaque)]
pub struct LayoutMap {
    tree: SumTree<LineBlock>,
}

impl LayoutMap {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            tree: SumTree::new(()),
        }
    }

    #[frb(sync)]
    pub fn push_line(&mut self, len_chars: usize, height: f32, is_folded: bool) {
        let mut new_tree = self.tree.clone();
        new_tree.push(LineBlock { len_chars, height, is_folded }, ());
        self.tree = new_tree;
    }

    #[frb(sync)]
    pub fn clear(&mut self) {
        self.tree = SumTree::new(());
    }
}


// -------------------------------------------------------------------------
// 2. FOLD RANGE & BRACKET MATCHING ENGINE
// -------------------------------------------------------------------------

pub struct RustFoldRange {
    pub start_line: i32,
    pub end_line: i32,
}

pub fn folds_compute_all(rope: &RopeBridge) -> Vec<RustFoldRange> {
    let mut folds = Vec::new();
    let mut stack: Vec<(char, i32)> = Vec::new();
    let mut line_idx: i32 = 0;

    let rope = rope.rope.read().unwrap();
    for ch in rope.chars() {
        if ch == '\n' {
            line_idx += 1;
        }
        if ch == '{' || ch == '[' || ch == '(' {
            stack.push((ch, line_idx));
        } else if ch == '}' || ch == ']' || ch == ')' {
            if let Some((open_ch, start_line)) = stack.pop() {
                let matches = match (open_ch, ch) {
                    ('{', '}') => true,
                    ('[', ']') => true,
                    ('(', ')') => true,
                    _ => false,
                };
                if matches && start_line < line_idx {
                    folds.push(RustFoldRange {
                        start_line,
                        end_line: line_idx,
                    });
                }
            }
        }
    }
    folds
}

#[frb(sync)]
pub fn folds_find_matching_bracket(rope: &RopeBridge, target_offset: i32) -> i64 {
    if target_offset < 0 {
        return -1;
    }
    let rope = rope.rope.read().unwrap();
    let len = rope.len_chars();
    let target_offset = target_offset as usize;
    if target_offset >= len {
        return -1;
    }

    let start_ch: char = rope.char(target_offset);

    let (matcher, search_forward) = match start_ch {
        '{' => ('}', true),
        '[' => (']', true),
        '(' => (')', true),
        '}' => ('{', false),
        ']' => ('[', false),
        ')' => ('(', false),
        _ => return -1,
    };

    let mut depth = 1;

    if search_forward {
        for (i, ch) in rope.chars_at(target_offset + 1).enumerate() {
            let idx = target_offset + 1 + i;
            if idx >= len {
                break;
            }
            if ch == start_ch {
                depth += 1;
            } else if ch == matcher {
                depth -= 1;
                if depth == 0 {
                    return idx as i64;
                }
            }
        }
    } else {
        if target_offset == 0 {
            return -1;
        }
        for idx in (0..target_offset).rev() {
            let ch = rope.char(idx);
            if ch == start_ch {
                depth += 1;
            } else if ch == matcher {
                depth -= 1;
                if depth == 0 {
                    return idx as i64;
                }
            }
        }
    }

    -1
}

#[derive(Clone, Debug)]
pub struct GuideBlock {
    pub start_line: i32,
    pub end_line: i32,
    pub indent_level: i32,
    pub leading_spaces: i32,
}

#[frb(sync)]
pub fn guides_compute_viewport(
    _rope: &RopeBridge,
    _first_visible: usize,
    _last_visible: usize,
    _tab_size: usize,
) -> Vec<GuideBlock> {
    let vec = Vec::new();
    // stub implementation
    vec
}

// -------------------------------------------------------------------------
// 3. WORD EXTRACTION ENGINE
// -------------------------------------------------------------------------

#[frb(sync)]
pub fn words_extract(rope: &RopeBridge) -> Vec<String> {
    let mut words = HashSet::new();
    let mut current_word = String::new();
    
    let text = rope.get_text();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                words.insert(current_word.clone());
                current_word.clear();
            }
        }
    }
    if !current_word.is_empty() {
        words.insert(current_word);
    }
    
    words.into_iter().collect()
}
