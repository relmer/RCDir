// tree_connector_state.rs — Tree drawing state for hierarchical directory display
//
// Tracks ancestor sibling relationships and generates Unicode box-drawing
// prefixes for tree view mode.  Passed by mutable reference through the
// display call chain as the main thread recurses into child directories.





////////////////////////////////////////////////////////////////////////////////

/// Lightweight struct tracking the tree drawing state as the main thread
/// recurses through the directory tree.
///
/// Port of: CTreeConnectorState (TCDir)
pub struct TreeConnectorState {
    /// One entry per nesting depth.  `true` = ancestor at that level has more
    /// siblings coming (draw `│`); `false` = ancestor was last (draw space).
    ancestor_has_sibling: Vec<bool>,

    /// Characters per indent level (from CommandLine::tree_indent, default 4).
    tree_indent: i32,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl TreeConnectorState
//
//  Tree connector state construction and prefix generation.
//
////////////////////////////////////////////////////////////////////////////////

impl TreeConnectorState {

    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new empty state at depth 0.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new (tree_indent: i32) -> Self {
        TreeConnectorState {
            ancestor_has_sibling: Vec::new(),
            tree_indent,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  push
    //
    //  Push a new depth level (entering a subdirectory).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn push (&mut self, has_sibling: bool) {
        self.ancestor_has_sibling.push (has_sibling);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pop
    //
    //  Pop a depth level (leaving a subdirectory).  No-op at depth 0.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn pop (&mut self) {
        self.ancestor_has_sibling.pop();
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  depth
    //
    //  Current nesting depth (length of ancestor_has_sibling).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn depth (&self) -> usize {
        self.ancestor_has_sibling.len()
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_prefix
    //
    //  Generate the full tree prefix string for the current entry.
    //  Iterates ancestor_has_sibling to build continuation lines, then
    //  appends `├── ` or `└── ` based on is_last_entry.
    //
    //  At depth 0 (root): returns empty string (no connectors).
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn get_prefix (&self, is_last_entry: bool) -> String {
        let depth = self.ancestor_has_sibling.len();
        if depth == 0 {
            return String::new();
        }

        let indent = self.tree_indent.max (1) as usize;
        let mut result = String::with_capacity (depth * indent + indent);

        // Ancestor continuation lines (skip index 0 = root level has
        // no visible connectors)
        for i in 1..depth {
            if self.ancestor_has_sibling[i] {
                // Ancestor has more siblings → draw vertical continuation
                result.push ('│');
            } else {
                // Ancestor was last → draw space
                result.push (' ');
            }
            // Fill remaining width with spaces
            for _ in 1..indent {
                result.push (' ');
            }
        }

        // Current level connector
        if is_last_entry {
            result.push ('└');
        } else {
            result.push ('├');
        }

        // Horizontal dashes: max(indent - 2, 0) dashes
        let dash_count = if indent > 2 { indent - 2 } else { 0 };
        for _ in 0..dash_count {
            result.push ('─');
        }

        // Always trailing space after connector
        result.push (' ');

        result
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  get_stream_continuation
    //
    //  Generate the prefix for a stream line: same as regular prefix but
    //  replaces the connector with `│` + padding (vertical continuation).
    //
    //  At depth 0 (root): returns vertical continuation for current level.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn get_stream_continuation (&self) -> String {
        let depth = self.ancestor_has_sibling.len();
        let indent = self.tree_indent.max (1) as usize;

        if depth == 0 {
            return String::new();
        }

        let mut result = String::with_capacity ((depth + 1) * indent);

        // Ancestor continuation lines (skip index 0 = root)
        for i in 1..depth {
            if self.ancestor_has_sibling[i] {
                result.push ('│');
            } else {
                result.push (' ');
            }
            for _ in 1..indent {
                result.push (' ');
            }
        }

        // Current level stream continuation (always vertical)
        result.push ('│');
        for _ in 1..indent {
            result.push (' ');
        }

        result
    }
}





#[cfg(test)]
mod tests {
    use super::*;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  default_constructor_depth0
    //
    //  Default TreeConnectorState has depth 0 and indent 4.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn default_constructor_depth0 () {
        let state = TreeConnectorState::new (4);
        assert_eq! (state.depth(), 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  custom_indent_stored_correctly
    //
    //  Custom indent value is stored correctly.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn custom_indent_stored_correctly () {
        let state = TreeConnectorState::new (2);
        assert_eq! (state.tree_indent, 2);
        assert_eq! (state.depth(), 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth0_empty_string
    //
    //  At depth 0, GetPrefix returns empty string (no connectors).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth0_empty_string () {
        let state = TreeConnectorState::new (4);
        assert_eq! (state.get_prefix (false), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth0_last_entry_empty_string
    //
    //  At depth 0, GetPrefix returns empty string even for last entry.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth0_last_entry_empty_string () {
        let state = TreeConnectorState::new (4);
        assert_eq! (state.get_prefix (true), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth1_middle_entry
    //
    //  At depth 1, middle entry gets ├── connector.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth1_middle_entry () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        assert_eq! (state.get_prefix (false), "├── ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth1_last_entry
    //
    //  At depth 1, last entry gets └── connector.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth1_last_entry () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        assert_eq! (state.get_prefix (true), "└── ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth2_middle_entry_ancestor_has_sibling
    //
    //  At depth 2 with ancestor that has siblings, shows │ continuation.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth2_middle_entry_ancestor_has_sibling () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        state.push (true);
        assert_eq! (state.get_prefix (false), "│   ├── ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth2_last_entry_ancestor_has_no_sibling
    //
    //  At depth 2 with ancestor that was last, shows └── with │ continuation
    //  from the first push.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth2_last_entry_ancestor_has_no_sibling () {
        let mut state = TreeConnectorState::new (4);
        state.push (false);
        state.push (true);
        assert_eq! (state.get_prefix (true), "│   └── ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  prefix_at_depth3_mixed_ancestors
    //
    //  At depth 3 with mixed ancestor sibling states.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn prefix_at_depth3_mixed_ancestors () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        state.push (false);
        state.push (true);
        assert_eq! (state.get_prefix (false), "    │   ├── ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  push_pop_depth_tracking
    //
    //  Push increments depth, Pop decrements.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn push_pop_depth_tracking () {
        let mut state = TreeConnectorState::new (4);
        assert_eq! (state.depth(), 0);
        state.push (true);
        assert_eq! (state.depth(), 1);
        state.push (false);
        assert_eq! (state.depth(), 2);
        state.pop();
        assert_eq! (state.depth(), 1);
        state.pop();
        assert_eq! (state.depth(), 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  pop_at_depth0_no_op
    //
    //  Pop at depth 0 is a no-op (doesn't crash).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn pop_at_depth0_no_op () {
        let mut state = TreeConnectorState::new (4);
        state.pop();
        assert_eq! (state.depth(), 0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stream_continuation_depth0_empty_string
    //
    //  Stream continuation at depth 0 returns empty string.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn stream_continuation_depth0_empty_string () {
        let state = TreeConnectorState::new (4);
        assert_eq! (state.get_stream_continuation(), "");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stream_continuation_depth1
    //
    //  Stream continuation at depth 1 shows │ + spaces.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn stream_continuation_depth1 () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        assert_eq! (state.get_stream_continuation(), "│   ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stream_continuation_depth2_ancestor_has_sibling
    //
    //  Stream continuation at depth 2 with ancestor siblings.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn stream_continuation_depth2_ancestor_has_sibling () {
        let mut state = TreeConnectorState::new (4);
        state.push (true);
        state.push (true);
        assert_eq! (state.get_stream_continuation(), "│   │   ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  custom_indent1_short_prefix
    //
    //  Indent=1: ├ + space (no horizontal dashes).
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn custom_indent1_short_prefix () {
        let mut state = TreeConnectorState::new (1);
        state.push (true);
        assert_eq! (state.get_prefix (false), "├ ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  custom_indent2_narrow_prefix
    //
    //  Indent=2 at depth 2: │ + space, then └ + space.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn custom_indent2_narrow_prefix () {
        let mut state = TreeConnectorState::new (2);
        state.push (true);
        state.push (true);
        assert_eq! (state.get_prefix (true), "│ └ ");
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  custom_indent8_wide_prefix
    //
    //  Indent=8 at depth 1: ├ + 6 horizontal dashes + space.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn custom_indent8_wide_prefix () {
        let mut state = TreeConnectorState::new (8);
        state.push (true);
        assert_eq! (state.get_prefix (false), "├────── ");
    }
}
