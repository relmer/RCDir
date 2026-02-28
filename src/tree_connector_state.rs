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
        let mut result = String::with_capacity (depth * indent);

        // Ancestor continuation lines (all levels except the current one)
        for i in 0..depth - 1 {
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

        // Horizontal dashes: indent - 2 dashes + 1 trailing space
        let dash_count = if indent >= 2 { indent - 2 } else { 0 };
        for _ in 0..dash_count {
            result.push ('─');
        }
        if indent >= 2 {
            result.push (' ');
        }

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

        // Ancestor continuation lines
        for i in 0..depth {
            if self.ancestor_has_sibling[i] {
                result.push ('│');
            } else {
                result.push (' ');
            }
            for _ in 1..indent {
                result.push (' ');
            }
        }

        result
    }
}
