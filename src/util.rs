use pad::PadStr;
use std::collections::HashMap;
use std::fmt::Formatter;

#[must_use]
pub struct Table {
    n_cols: usize,
    widths: Vec<usize>,
    data: Vec<Vec<String>>,
}
impl Table {
    pub fn new(header: Vec<String>) -> Table {
        Table {
            n_cols: header.len(),
            widths: header.iter().map(String::len).collect(),
            data: vec![header],
        }
    }
    pub fn add_row(&mut self, row: Vec<String>) {
        assert_eq!(self.n_cols, row.len());
        self.data.push(row);
        self.update_widths();
    }
    fn update_widths(&mut self) {
        for (i, s) in self.data.last().expect("expected last row to exist").iter().enumerate() {
            if s.len() > self.widths[i] {
                self.widths[i] = s.len();
            }
        }
    }
}
impl std::fmt::Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for row in self.data.iter() {
            for i in 0..row.len() {
                write!(f, "{}  ", row[i].pad_to_width(self.widths[i]))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}