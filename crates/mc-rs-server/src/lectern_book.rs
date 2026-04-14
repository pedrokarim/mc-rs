//! Lectern — displays written books, broadcasts redstone pages.

#[derive(Debug, Clone)]
pub struct Lectern {
    pub book_placed: Option<LecternBook>,
    pub facing: u8,
    pub current_page: u16,
    pub max_pages: u16,
}

#[derive(Debug, Clone)]
pub struct LecternBook {
    pub title: String,
    pub author: String,
    pub pages: Vec<String>,
}

impl Lectern {
    pub fn new(facing: u8) -> Self {
        Self { book_placed: None, facing, current_page: 0, max_pages: 0 }
    }

    pub fn place_book(&mut self, book: LecternBook) -> bool {
        if self.book_placed.is_some() {
            return false;
        }
        self.max_pages = book.pages.len() as u16;
        self.current_page = 0;
        self.book_placed = Some(book);
        true
    }

    pub fn take_book(&mut self) -> Option<LecternBook> {
        self.current_page = 0;
        self.max_pages = 0;
        self.book_placed.take()
    }

    pub fn turn_page(&mut self, next: bool) {
        if next && self.current_page + 1 < self.max_pages {
            self.current_page += 1;
        } else if !next && self.current_page > 0 {
            self.current_page -= 1;
        }
    }

    /// Redstone output = (current_page + 1) * 15 / max_pages.
    pub fn comparator_output(&self) -> u8 {
        if self.max_pages == 0 {
            return 0;
        }
        ((self.current_page as u32 + 1) * 15 / self.max_pages as u32).min(15) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_no_output() {
        let l = Lectern::new(0);
        assert_eq!(l.comparator_output(), 0);
    }
}
