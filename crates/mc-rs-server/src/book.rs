//! Book system — port PMMP `src/item/WritableBook.php` + `WrittenBook.php`.

pub const MAX_BOOK_PAGES: usize = 50;
pub const MAX_PAGE_LENGTH: usize = 798; // vanilla Bedrock

#[derive(Debug, Clone, Default)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub xuid: String,
    pub generation: u8, // 0=original 1=copy 2=copy_of_copy 3=tattered
    pub pages: Vec<String>,
    pub signed: bool,
}

impl Book {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_page(&mut self, text: impl Into<String>) -> bool {
        if self.signed {
            return false;
        }
        let mut page = text.into();
        if page.len() > MAX_PAGE_LENGTH {
            page.truncate(MAX_PAGE_LENGTH);
        }
        if self.pages.len() >= MAX_BOOK_PAGES {
            return false;
        }
        self.pages.push(page);
        true
    }

    pub fn edit_page(&mut self, index: usize, text: impl Into<String>) -> bool {
        if self.signed {
            return false;
        }
        if index >= self.pages.len() {
            return false;
        }
        let mut page = text.into();
        if page.len() > MAX_PAGE_LENGTH {
            page.truncate(MAX_PAGE_LENGTH);
        }
        self.pages[index] = page;
        true
    }

    pub fn remove_page(&mut self, index: usize) -> bool {
        if self.signed || index >= self.pages.len() {
            return false;
        }
        self.pages.remove(index);
        true
    }

    pub fn sign(&mut self, title: impl Into<String>, author: impl Into<String>, xuid: impl Into<String>) -> bool {
        if self.signed {
            return false;
        }
        self.title = title.into();
        self.author = author.into();
        self.xuid = xuid.into();
        self.signed = true;
        true
    }

    pub fn is_writable(&self) -> bool {
        !self.signed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_book_cannot_edit() {
        let mut b = Book::new();
        b.add_page("Chapter 1");
        b.sign("My Tales", "Alice", "xuid123");
        assert!(!b.add_page("Chapter 2"));
        assert!(!b.edit_page(0, "modified"));
    }

    #[test]
    fn long_page_truncated() {
        let mut b = Book::new();
        let long = "a".repeat(1000);
        b.add_page(long);
        assert_eq!(b.pages[0].len(), MAX_PAGE_LENGTH);
    }

    #[test]
    fn max_pages_respected() {
        let mut b = Book::new();
        for i in 0..60 {
            b.add_page(format!("page {i}"));
        }
        assert_eq!(b.pages.len(), MAX_BOOK_PAGES);
    }
}
