// Vaflo – A word game in Esperanto
// Copyright (C) 2023  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::{Mutex, Condvar};
use super::grid::Grid;

pub enum SolverState {
    Idle,
    Task { grid_id: usize, grid: Grid },
    Quit,
}

pub struct SolverStatePair {
    state: Mutex<SolverState>,
    condvar: Condvar,
}

impl SolverStatePair {
    pub fn new() -> SolverStatePair {
        SolverStatePair {
            state: Mutex::new(SolverState::Idle),
            condvar: Condvar::new(),
        }
    }

    pub fn later_task_is_pending(
        &self,
        completed_grid_id: Option<usize>,
    ) -> bool {
        match *self.state.lock().unwrap() {
            SolverState::Idle => false,
            SolverState::Task { grid_id, .. } => {
                completed_grid_id.map(|id| id < grid_id).unwrap_or(true)
            },
            SolverState::Quit => true,
        }
    }

    pub fn wait(&self, completed_grid_id: Option<usize>) -> SolverState {
        let mut state = self.state.lock().unwrap();

        loop {
            match *state {
                SolverState::Idle => state = self.condvar.wait(state).unwrap(),
                SolverState::Task { grid_id, ref grid } => {
                    if completed_grid_id.map(|id| id < grid_id)
                        .unwrap_or(true)
                    {
                        break SolverState::Task {
                            grid_id,
                            grid: grid.clone(),
                        };
                    } else {
                        state = self.condvar.wait(state).unwrap();
                    }
                },
                SolverState::Quit => break SolverState::Quit,
            }
        }
    }

    pub fn set_grid(&self, grid_id: usize, grid: Grid) {
        let mut state = self.state.lock().unwrap();
        *state = SolverState::Task { grid_id, grid };
        self.condvar.notify_all();
    }

    pub fn quit(&self) {
        *self.state.lock().unwrap() = SolverState::Quit;
        self.condvar.notify_all();
    }
}
