//! A dense rectangular grid and the cell coordinate everything else speaks in.

use std::fmt;

/// A cell position, `x` to the right and `y` downwards, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
}

impl Cell {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// A width × height array of `T`, stored row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<T> {
    width: u32,
    height: u32,
    cells: Vec<T>,
}

/// The grid could not be built from the pieces offered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridError {
    /// A zero-sized grid is never what the caller meant.
    Empty { width: u32, height: u32 },
    /// `cells.len()` did not equal `width * height`.
    LengthMismatch { width: u32, height: u32, got: usize },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GridError::Empty { width, height } => {
                write!(f, "a {width}x{height} grid has no cells")
            }
            GridError::LengthMismatch { width, height, got } => write!(
                f,
                "a {width}x{height} grid needs {} cells, got {got}",
                u64::from(*width) * u64::from(*height)
            ),
        }
    }
}

impl std::error::Error for GridError {}

impl<T> Grid<T> {
    pub fn from_vec(width: u32, height: u32, cells: Vec<T>) -> Result<Self, GridError> {
        if width == 0 || height == 0 {
            return Err(GridError::Empty { width, height });
        }
        let expected = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(usize::MAX);
        if cells.len() != expected {
            return Err(GridError::LengthMismatch {
                width,
                height,
                got: cells.len(),
            });
        }
        Ok(Self {
            width,
            height,
            cells,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Always false: [`Grid::from_vec`] rejects zero-sized grids.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn contains(&self, cell: Cell) -> bool {
        cell.x < self.width && cell.y < self.height
    }

    fn index(&self, cell: Cell) -> Option<usize> {
        if !self.contains(cell) {
            return None;
        }
        Some(cell.y as usize * self.width as usize + cell.x as usize)
    }

    pub fn get(&self, cell: Cell) -> Option<&T> {
        self.index(cell).map(|i| &self.cells[i])
    }

    pub fn get_mut(&mut self, cell: Cell) -> Option<&mut T> {
        self.index(cell).map(move |i| &mut self.cells[i])
    }

    pub fn set(&mut self, cell: Cell, value: T) -> bool {
        match self.index(cell) {
            Some(i) => {
                self.cells[i] = value;
                true
            }
            None => false,
        }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.cells
    }

    pub fn into_vec(self) -> Vec<T> {
        self.cells
    }

    /// Every cell in reading order: left to right, top to bottom.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        let width = self.width;
        let height = self.height;
        (0..height).flat_map(move |y| (0..width).map(move |x| Cell::new(x, y)))
    }

    /// Every `(position, value)` pair in reading order.
    pub fn iter(&self) -> impl Iterator<Item = (Cell, &T)> + '_ {
        self.cells().map(move |cell| {
            let index = cell.y as usize * self.width as usize + cell.x as usize;
            (cell, &self.cells[index])
        })
    }

    /// The four edge-sharing neighbours that are inside the grid.
    pub fn neighbours4(&self, cell: Cell) -> Vec<Cell> {
        let mut out = Vec::with_capacity(4);
        if cell.y > 0 {
            out.push(Cell::new(cell.x, cell.y - 1));
        }
        if cell.x > 0 {
            out.push(Cell::new(cell.x - 1, cell.y));
        }
        if cell.x + 1 < self.width {
            out.push(Cell::new(cell.x + 1, cell.y));
        }
        if cell.y + 1 < self.height {
            out.push(Cell::new(cell.x, cell.y + 1));
        }
        out
    }

    pub fn map<U>(&self, mut f: impl FnMut(Cell, &T) -> U) -> Grid<U> {
        let cells = self.iter().map(|(cell, value)| f(cell, value)).collect();
        Grid {
            width: self.width,
            height: self.height,
            cells,
        }
    }
}

impl<T: Clone> Grid<T> {
    pub fn filled(width: u32, height: u32, value: T) -> Result<Self, GridError> {
        if width == 0 || height == 0 {
            return Err(GridError::Empty { width, height });
        }
        let count = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(usize::MAX);
        Ok(Self {
            width,
            height,
            cells: vec![value; count],
        })
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn rejects_a_length_that_does_not_match() {
        let error = Grid::from_vec(2, 2, vec![1u8, 2, 3]).unwrap_err();
        assert_eq!(
            error,
            GridError::LengthMismatch {
                width: 2,
                height: 2,
                got: 3
            }
        );
    }

    #[test]
    fn reads_in_reading_order() {
        let grid = Grid::from_vec(2, 2, vec![1u8, 2, 3, 4]).expect("2x2");
        let seen: Vec<u8> = grid.iter().map(|(_, v)| *v).collect();
        assert_eq!(seen, vec![1, 2, 3, 4]);
        assert_eq!(grid.get(Cell::new(1, 0)), Some(&2));
        assert_eq!(grid.get(Cell::new(0, 1)), Some(&3));
        assert_eq!(grid.get(Cell::new(2, 0)), None);
    }

    #[test]
    fn neighbours_stop_at_the_edge() {
        let grid = Grid::filled(3, 3, 0u8).expect("3x3");
        assert_eq!(grid.neighbours4(Cell::new(0, 0)).len(), 2);
        assert_eq!(grid.neighbours4(Cell::new(1, 1)).len(), 4);
        assert_eq!(grid.neighbours4(Cell::new(2, 2)).len(), 2);
    }
}
