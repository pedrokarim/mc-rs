//! Lectern — port PMMP `src/block/tile/Lectern.php`.
//! Support de livre qui émet redstone selon la page visitée.

use crate::book::Book;

#[derive(Debug, Clone, Default)]
pub struct LecternState {
    pub book: Option<Book>,
    pub current_page: u32,
}

impl LecternState {
    pub fn put_book(&mut self, book: Book) -> bool {
        if self.book.is_some() {
            return false;
        }
        self.book = Some(book);
        self.current_page = 0;
        true
    }

    pub fn take_book(&mut self) -> Option<Book> {
        self.current_page = 0;
        self.book.take()
    }

    /// Change page. Retourne true si succeed.
    pub fn set_page(&mut self, page: u32) -> bool {
        match &self.book {
            Some(b) if (page as usize) < b.pages.len() => {
                self.current_page = page;
                true
            }
            _ => false,
        }
    }

    /// Signal redstone émis (0-15) selon progression dans le livre.
    pub fn redstone_signal(&self) -> u8 {
        match &self.book {
            None => 0,
            Some(b) if b.pages.is_empty() => 0,
            Some(b) => {
                let progress = self.current_page as f32 / b.pages.len() as f32;
                (progress * 15.0).round() as u8
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_at_end_is_15() {
        let mut l = LecternState::default();
        let mut b = Book::new();
        for i in 0..10 {
            b.add_page(format!("page {i}"));
        }
        l.put_book(b);
        l.set_page(9);
        assert!(l.redstone_signal() > 12);
    }

    #[test]
    fn cannot_put_two_books() {
        let mut l = LecternState::default();
        let b1 = Book::new();
        let b2 = Book::new();
        assert!(l.put_book(b1));
        assert!(!l.put_book(b2));
    }
}
