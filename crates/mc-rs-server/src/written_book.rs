//! Written book — author, pages, title.

#[derive(Debug, Clone)]
pub struct WrittenBook {
    pub title: String,
    pub author: String,
    pub pages: Vec<String>,
    pub generation: u8, // 0=original, 1=copy, 2=copy_of_copy, 3=tattered
    pub resolved: bool,
}

/// Max pages.
pub const MAX_PAGES: usize = 100;
/// Max title length.
pub const MAX_TITLE_LENGTH: usize = 16;
/// Max text per page.
pub const MAX_PAGE_LENGTH: usize = 256;

impl WrittenBook {
    pub fn new(title: String, author: String, pages: Vec<String>) -> Self {
        Self {
            title: title.chars().take(MAX_TITLE_LENGTH).collect(),
            author,
            pages: pages
                .into_iter()
                .take(MAX_PAGES)
                .map(|p| p.chars().take(MAX_PAGE_LENGTH).collect())
                .collect(),
            generation: 0,
            resolved: false,
        }
    }

    /// Copy book (generation + 1).
    pub fn copy(&self) -> Option<Self> {
        if self.generation >= 2 {
            return None;
        }
        let mut new_book = self.clone();
        new_book.generation += 1;
        Some(new_book)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_copy_twice() {
        let b = WrittenBook::new("t".into(), "a".into(), vec!["p".into()]);
        let c1 = b.copy().unwrap();
        let c2 = c1.copy();
        assert!(c2.is_some());
        assert!(c2.unwrap().copy().is_none());
    }
}
