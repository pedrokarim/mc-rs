//! Helpers comparing block states.

/// Check if two blocks (id + data) are effectively the same.
pub fn same_block(a: (u16, u16), b: (u16, u16)) -> bool {
    a.0 == b.0 && a.1 == b.1
}

/// Group blocks by "material" for dev scripting (crafting substitutions).
pub fn wood_variants() -> &'static [&'static str] {
    &["oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry", "bamboo"]
}

pub fn stone_variants() -> &'static [&'static str] {
    &["stone", "granite", "diorite", "andesite", "deepslate", "tuff", "calcite"]
}

pub fn sand_variants() -> &'static [&'static str] {
    &["sand", "red_sand"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_list_non_empty() {
        assert!(!wood_variants().is_empty());
    }
}
