use crossterm::event::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Mode {
    TableSelect,
    RowSelect,
}

#[derive(Debug, Clone, Copy)]
pub struct CliState {
    pub(crate) table_idx: usize,
    pub(crate) row_idx: usize,
    pub(crate) num_tables: usize,
    pub(crate) num_rows: usize,
    pub(crate) mode: Mode,
}

pub enum Signal {
    Exit,
    UpdateTable,
    UpdateRow,
}

struct Active<'a> {
    idx: &'a mut usize,
    limit: usize,
    signal: Signal,
}

fn active(state: &mut CliState) -> Active<'_> {
    match state.mode {
        Mode::TableSelect => Active {
            idx: &mut state.table_idx,
            limit: state.num_tables,
            signal: Signal::UpdateTable,
        },
        Mode::RowSelect => Active {
            idx: &mut state.row_idx,
            limit: state.num_rows,
            signal: Signal::UpdateRow,
        },
    }
}

fn handle_up(state: &mut CliState) -> Option<Signal> {
    let active = active(state);
    if *active.idx > 0 {
        *active.idx -= 1;
    }
    Some(active.signal)
}

fn handle_down(state: &mut CliState) -> Option<Signal> {
    let active = active(state);
    if *active.idx < active.limit - 1 {
        *active.idx += 1;
    }
    Some(active.signal)
}

fn handle_left(state: &mut CliState) -> Option<Signal> {
    match state.mode {
        Mode::TableSelect => None,
        Mode::RowSelect => {
            state.mode = Mode::TableSelect;
            Some(Signal::UpdateTable)
        }
    }
}

fn handle_right(state: &mut CliState) -> Option<Signal> {
    match state.mode {
        Mode::TableSelect => {
            state.mode = Mode::RowSelect;
            Some(Signal::UpdateRow)
        }
        Mode::RowSelect => None,
    }
}

fn handle_enter(state: &mut CliState) -> Option<Signal> {
    match state.mode {
        Mode::TableSelect => {
            state.mode = Mode::RowSelect;
            Some(Signal::UpdateRow)
        }
        Mode::RowSelect => None,
    }
}

pub fn handle_key(state: &mut CliState, key: KeyCode) -> Option<Signal> {
    match key {
        KeyCode::Up => handle_up(state),
        KeyCode::Down => handle_down(state),
        KeyCode::Enter => handle_enter(state),
        KeyCode::Left => handle_left(state),
        KeyCode::Right => handle_right(state),
        KeyCode::Esc => Some(Signal::Exit), // quit
        _ => None,
    }
}
