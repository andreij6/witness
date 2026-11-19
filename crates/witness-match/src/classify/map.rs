#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CellState {
    Uncompared = 0,
    Agree = 1,
    Changed = 2,
}

pub trait ChangeMap {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn state(&self, x: u32, y: u32) -> CellState;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateMap {
    width: u32,
    height: u32,
    states: Vec<CellState>,
}

impl StateMap {
    pub fn new(width: u32, height: u32, states: Vec<CellState>) -> Option<Self> {
        (states.len() as u64 == width as u64 * height as u64).then_some(Self {
            width,
            height,
            states,
        })
    }

    pub fn filled(width: u32, height: u32, state: CellState) -> Self {
        Self {
            width,
            height,
            states: vec![state; width as usize * height as usize],
        }
    }

    pub fn from_fn(width: u32, height: u32, mut f: impl FnMut(u32, u32) -> CellState) -> Self {
        let mut states = Vec::with_capacity(width as usize * height as usize);
        for y in 0..height {
            for x in 0..width {
                states.push(f(x, y));
            }
        }
        Self {
            width,
            height,
            states,
        }
    }

    pub fn states(&self) -> &[CellState] {
        &self.states
    }

    pub fn set(&mut self, x: u32, y: u32, state: CellState) {
        if x < self.width && y < self.height {
            self.states[y as usize * self.width as usize + x as usize] = state;
        }
    }

    pub fn count(&self, state: CellState) -> u64 {
        self.states.iter().filter(|&&s| s == state).count() as u64
    }
}

impl ChangeMap for StateMap {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn state(&self, x: u32, y: u32) -> CellState {
        self.states[y as usize * self.width as usize + x as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridShape {
    pub cols: u32,
    pub rows: u32,
}

impl GridShape {
    pub const LANDSCAPE: GridShape = GridShape { cols: 16, rows: 12 };
    pub const PORTRAIT: GridShape = GridShape { cols: 12, rows: 16 };

    pub fn for_frame(width: u32, height: u32) -> GridShape {
        if width >= height {
            Self::LANDSCAPE
        } else {
            Self::PORTRAIT
        }
    }

    pub fn cell_size(&self, width: u32, height: u32) -> (f64, f64) {
        (
            width as f64 / self.cols.max(1) as f64,
            height as f64 / self.rows.max(1) as f64,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_validates_length() {
        assert!(StateMap::new(2, 2, vec![CellState::Agree; 4]).is_some());
        assert!(StateMap::new(2, 2, vec![CellState::Agree; 3]).is_none());
        assert!(StateMap::new(0, 5, vec![]).is_some());
    }

    #[test]
    fn from_fn_and_access() {
        let map = StateMap::from_fn(3, 2, |x, y| {
            if x == y {
                CellState::Changed
            } else {
                CellState::Agree
            }
        });
        assert_eq!(map.state(0, 0), CellState::Changed);
        assert_eq!(map.state(1, 1), CellState::Changed);
        assert_eq!(map.state(2, 1), CellState::Agree);
        assert_eq!(map.count(CellState::Changed), 2);
        assert_eq!((map.width(), map.height()), (3, 2));
        assert_eq!(map.states().len(), 6);
    }

    #[test]
    fn set_ignores_out_of_range() {
        let mut map = StateMap::filled(2, 2, CellState::Uncompared);
        map.set(1, 1, CellState::Changed);
        map.set(5, 5, CellState::Changed);
        assert_eq!(map.count(CellState::Changed), 1);
        assert_eq!(map.count(CellState::Uncompared), 3);
    }

    #[test]
    fn grid_orientation_follows_frame() {
        assert_eq!(GridShape::for_frame(1600, 1200), GridShape::LANDSCAPE);
        assert_eq!(GridShape::for_frame(1200, 1600), GridShape::PORTRAIT);
        assert_eq!(GridShape::for_frame(100, 100), GridShape::LANDSCAPE);
        assert_eq!(GridShape::LANDSCAPE.cell_size(1600, 1200), (100.0, 100.0));
        assert_eq!(
            GridShape { cols: 0, rows: 0 }.cell_size(10, 10),
            (10.0, 10.0)
        );
    }
}
