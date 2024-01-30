use atb::prelude::*;
use atb_types::Uuid;
use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::IntoEnumIterator;

use crate::game_core::config::{
    Bead, ClearPattern, Element, BOARD_HEIGHT, BOARD_NUM_COLORS, BOARD_WIDTH,
};
use crate::game_core::event_module::GamerMove;
use crate::game_core::skill::SkillInfo;
use crate::game_core::GameError;

const MASK_OFFSET: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BoardState {
    ClearState {
        #[serde(rename = "clearMask")]
        clear_mask: Vec<u32>, // Clear mask in response DO NOT have an offset (>> MASK_OFFSET)
        combo_states: Vec<ComboState>,
    },
    FillState {
        board: Board,
    },
    RerollState {
        board: Board,
    },
    TurnTilesState {
        #[serde(rename = "clearMask")]
        clear_mask: Vec<u32>,
        board: Board,
    },
}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug, Clone, Copy)]
#[repr(i8)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WaitAction {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerAction {
    pub move_action: Option<MoveAction>,
    pub skill_action: Option<SkillAction>,
    pub wait_action: Option<WaitAction>,
}

impl Default for PlayerAction {
    fn default() -> Self {
        PlayerAction {
            move_action: None,
            skill_action: None,
            wait_action: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ComboState {
    pub color: usize,
    pub amount: u32,
    pub character_val_display: Vec<ClearValueDisplay>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClearValueDisplay {
    pub id: Uuid,
    pub damage: u32,
    pub cd_added: u32,   // CD added in each ClearState
    pub cd_charged: u32, // Total CD charged
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct MoveAction {
    pub x: u32,
    pub y: u32,
    pub direction: Direction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillAction {
    pub skill_info: SkillInfo,
    pub caster_id: Uuid,
    pub targets_id: Option<Vec<Uuid>>, // Some skill has no target, or have multiple targets
}

impl Default for SkillAction {
    fn default() -> Self {
        SkillAction {
            skill_info: SkillInfo::None,
            caster_id: Default::default(),
            targets_id: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub board_data: BoardData,
    board_field_mask: u32,
    wall_mask: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardData {
    width: u32,
    height: u32,
    num_colors: u32,
    pub board: Vec<Vec<u32>>, // [color][row_mask]
    pub remove_bead: Vec<u32>,
}

impl Board {
    pub fn new(rng: &mut StdRng, num_colors: u32, width: u32, height: u32) -> Self {
        let board_field_mask = !(u32::MAX << (width + MASK_OFFSET) | 1);
        let wall_mask = 1 << (width + MASK_OFFSET) | 1;
        let row = vec![0; height as usize * 2];
        let board = vec![row; num_colors as usize];

        let mut new_board = Board {
            board_data: BoardData {
                width,
                height,
                num_colors,
                board,
                remove_bead: vec![0; num_colors as usize],
            },
            board_field_mask,
            wall_mask,
        };
        new_board.refresh_reserved_block(0, rng);

        //NOTE: remove match beads before player move
        loop {
            match new_board.eval_clear_result(false) {
                None => break,
                Some((clear_mask, _)) => {
                    new_board.apply_clear_mask(clear_mask);
                    new_board.shift_empty();
                    new_board.refresh_reserved_block(height, rng);
                }
            }
        }
        while new_board.has_no_moves() {
            new_board.refresh_reserved_block(0, rng);
        }
        new_board
    }

    pub fn simulate(
        &mut self,
        _move: &MoveAction,
        rng: &mut StdRng,
    ) -> Result<Vec<BoardState>, GameError> {
        //update board move
        let mut next_board = self.clone();
        next_board.do_bead_swap(_move)?;

        //reset remove_bead
        for color in 0..next_board.board_data.num_colors {
            next_board.board_data.remove_bead[color as usize] = 0;
        }

        let states = next_board.process_falling_and_filling_result(rng);
        self.board_data.board = next_board.board_data.board;

        if states.len() == 0 {
            return Err(GameError::IllegalMove);
        }
        Ok(states)
    }

    pub fn has_valid_gem_target(&self, target_gem: Bead) -> bool {
        self.board_data.board[target_gem as usize]
            .iter()
            .take(self.board_data.height as usize)
            .sum::<u32>()
            != 0
    }

    pub fn remaining_colors_on_board(&self) -> Vec<Element> {
        Element::iter()
            .filter(|&element| element != Element::Unknown)
            .filter(|&element| {
                self.board_data.board[element as usize]
                    .iter()
                    .take(self.board_data.height as usize)
                    .sum::<u32>()
                    != 0
            })
            .collect()
    }

    pub fn turn_tiles(
        &mut self,
        from_elem: Element,
        to_elem: Element,
    ) -> Result<Vec<BoardState>, GameError> {
        let from_bead = Bead::from(from_elem);
        let to_bead = Bead::from(to_elem);

        if !self.has_valid_gem_target(from_bead) {
            return Err(GameError::SkillNoGemToTrigger);
        }

        let mut orig_color_board = self.board_data.board[from_bead as usize].clone();
        let mut dest_color_board = self.board_data.board[to_bead as usize].clone();
        let mut clear_mask = Vec::new();

        for row in 0..self.board_data.height as usize {
            dest_color_board[row] |= orig_color_board[row];
            clear_mask.push(orig_color_board[row] >> MASK_OFFSET);
            orig_color_board[row] = 0;
        }

        self.board_data.board[from_bead as usize] = orig_color_board;
        self.board_data.board[to_bead as usize] = dest_color_board;

        let mut states = Vec::new();
        states.push(BoardState::TurnTilesState {
            clear_mask,
            board: self.clone(),
        });

        return Ok(states);
    }

    pub fn element_explosion(
        &mut self,
        target_element: Element,
        rng: &mut StdRng,
    ) -> Result<Vec<BoardState>, GameError> {
        let target_bead = Bead::from(target_element);
        let target_color_board = self.board_data.board[target_bead as usize].clone();
        let mut board_clear_mask = vec![];

        // Find the beads to be clear
        let mut clear_bead_cnt = 0;
        for row in 0..self.board_data.height as usize {
            clear_bead_cnt += target_color_board[row].count_ones();
            board_clear_mask.push(target_color_board[row]);
        }
        if clear_bead_cnt == 0 {
            return Err(GameError::SkillNoGemToTrigger);
        };

        // Clear beads by skill triggering
        let mut states = vec![];
        states.push(BoardState::ClearState {
            clear_mask: board_clear_mask.iter().map(|v| v >> MASK_OFFSET).collect(),
            combo_states: Self::eval_skill_combo_state(
                &self.board_data.board,
                &board_clear_mask,
                SkillInfo::ElementalExplosion,
            ),
        });

        let mut next_board = self.clone();
        next_board.apply_clear_mask(board_clear_mask);
        next_board.shift_empty();
        next_board.refresh_reserved_block(self.board_data.height, rng);
        states.push(BoardState::FillState {
            board: next_board.clone(),
        });

        // Handle remaining falling and filling
        states.extend(next_board.process_falling_and_filling_result(rng));
        self.board_data.board = next_board.board_data.board;

        Ok(states)
    }

    pub fn line_eleminate(
        &mut self,
        clear_pattern: ClearPattern,
        line_num: u32,
        rng: &mut StdRng,
    ) -> Result<Vec<BoardState>, GameError> {
        let board_clear_mask = Self::compose_line_mask(clear_pattern, line_num);

        // Clear beads by skill triggering
        let mut states = vec![];
        states.push(BoardState::ClearState {
            clear_mask: board_clear_mask.iter().map(|v| v >> MASK_OFFSET).collect(),
            combo_states: Self::eval_skill_combo_state(
                &self.board_data.board,
                &board_clear_mask,
                SkillInfo::LineEliminate,
            ),
        });

        let mut next_board = self.clone();
        next_board.apply_clear_mask(board_clear_mask);
        next_board.shift_empty();
        next_board.refresh_reserved_block(self.board_data.height, rng);
        states.push(BoardState::FillState {
            board: next_board.clone(),
        });

        // Handle remaining falling and filling
        states.extend(next_board.process_falling_and_filling_result(rng));
        self.board_data.board = next_board.board_data.board;

        Ok(states)
    }

    fn compose_line_mask(clear_pattern: ClearPattern, line_num: u32) -> Vec<u32> {
        let mut line_mask = vec![0; BOARD_HEIGHT as usize];
        match clear_pattern {
            ClearPattern::Horizontal => line_mask[line_num as usize] = 255 << MASK_OFFSET,
            ClearPattern::Vertical => line_mask
                .iter_mut()
                .for_each(|x| *x = 1 << (line_num + MASK_OFFSET)),
            _ => unreachable!(),
        }

        line_mask
    }

    fn eval_skill_combo_state(
        board: &Vec<Vec<u32>>,
        clear_mask: &[u32],
        skill_info: SkillInfo,
    ) -> Vec<ComboState> {
        let mut combo_state = vec![];
        if !skill_info.is_clear_bead_produce_damage() {
            combo_state.push(ComboState::default());
            return combo_state;
        }

        // Count the amount of each color beads which are cleared.
        let amount: Vec<u32> = (0..BOARD_NUM_COLORS as usize)
            .map(|color| {
                board[color]
                    .iter()
                    .take(BOARD_HEIGHT as usize)
                    .enumerate()
                    .fold(0, |accu, (idx, row)| {
                        accu + (row & clear_mask[idx]).count_ones()
                    })
            })
            .collect();

        for color in 0..BOARD_NUM_COLORS as usize {
            if amount[color] != 0 {
                combo_state.push(ComboState {
                    color,
                    amount: amount[color],
                    character_val_display: vec![],
                });
            }
        }

        combo_state
    }

    pub fn get_move_info(&self, _move: &MoveAction) -> GamerMove {
        let board = self.board_data.board.clone();
        let x_mask = 1 << (_move.x + MASK_OFFSET);
        let mut selected_elem = Element::Unknown;
        for (color_idx, row_mask_list) in board.iter().enumerate() {
            if row_mask_list[_move.y as usize] & x_mask != 0 {
                selected_elem = Element::from(color_idx as u32);
                break;
            }
        }

        GamerMove {
            elem: selected_elem,
            clear_pattern: ClearPattern::Free, // ##TODO: Clear pattern related SPEC is not designed yet
        }
    }

    fn refresh_reserved_block(&mut self, starting_row: u32, rng: &mut StdRng) {
        let rows = self.board_data.board[0].len();

        // At (i,j) position, fill in a random color.
        // All reserved blocks above starting row will be refresh
        for i in starting_row as usize..rows {
            for color in 0..self.board_data.board.len() {
                self.board_data.board[color][i as usize] = 0;
            }
            for j in 0..self.board_data.width {
                //#FIXME we can efficiently stream the random using modulo chunks of each random
                //byte
                let color = rng.gen_range(0..self.board_data.num_colors);
                self.board_data.board[color as usize][i] =
                    self.board_data.board[color as usize][i] | 1 << (j + MASK_OFFSET);
            }
        }
    }

    fn do_bead_swap(&mut self, _move: &MoveAction) -> Result<(), GameError> {
        let row_mask = 1 << (_move.x + MASK_OFFSET);
        let mut dest_mask = row_mask;
        let mut dest_y = _move.y;
        match _move.direction {
            Direction::Right => {
                if _move.x >= (BOARD_WIDTH - 1) {
                    log::error!("Out of boundary: Right!");
                    return Err(GameError::IllegalMove);
                }
                dest_mask = dest_mask << 1
            }
            Direction::Left => {
                if _move.x == 0 {
                    log::error!("Out of boundary: Left!");
                    return Err(GameError::IllegalMove);
                }
                dest_mask = dest_mask >> 1
            }
            Direction::Up => {
                if dest_y >= (BOARD_HEIGHT - 1) {
                    log::error!("Out of boundary: Up!");
                    return Err(GameError::IllegalMove);
                }
                dest_y = dest_y + 1
            }
            Direction::Down => {
                if dest_y == 0 {
                    log::error!("Out of boundary: Down!");
                    return Err(GameError::IllegalMove);
                }
                dest_y = dest_y - 1
            }
        };

        for color in 0..self.board_data.board.len() {
            if dest_y == _move.y {
                // horizontal
                let mut extracted_orig_bit =
                    self.board_data.board[color][_move.y as usize] & row_mask;
                let mut extracted_dest_bit =
                    self.board_data.board[color][_move.y as usize] & dest_mask;
                match _move.direction {
                    Direction::Left => {
                        extracted_orig_bit = extracted_orig_bit >> 1;
                        extracted_dest_bit = extracted_dest_bit << 1;
                    }
                    _ => {
                        extracted_orig_bit = extracted_orig_bit << 1;
                        extracted_dest_bit = extracted_dest_bit >> 1;
                    }
                };
                self.board_data.board[color][_move.y as usize] =
                    self.board_data.board[color][_move.y as usize] & !(row_mask | dest_mask)
                        | extracted_orig_bit
                        | extracted_dest_bit;
            } else {
                //vertical
                let extracted_orig_bit = self.board_data.board[color][_move.y as usize] & row_mask;
                let extracted_dest_bit = self.board_data.board[color][dest_y as usize] & dest_mask;
                self.board_data.board[color][_move.y as usize] =
                    self.board_data.board[color][_move.y as usize] & !(row_mask | dest_mask)
                        | extracted_dest_bit;
                self.board_data.board[color][dest_y as usize] =
                    self.board_data.board[color][dest_y as usize] & !(row_mask | dest_mask)
                        | extracted_orig_bit;
            }
        }

        Ok(())
    }

    fn shift_empty(&mut self) {
        //#FIXME store/cache actual board height
        for x in 1..self.board_data.height * 2 {
            for y in (1..=x).rev() {
                let next_index = y - 1;
                let next_row = !(self.find_hollow_in_row(next_index));
                for color in 0..self.board_data.board.len() {
                    let drop_blocks = next_row & self.board_data.board[color][y as usize];
                    self.board_data.board[color][next_index as usize] =
                        drop_blocks | self.board_data.board[color][next_index as usize];
                    self.board_data.board[color][y as usize] =
                        !drop_blocks & self.board_data.board[color][y as usize];
                }
            }
        }
    }

    fn find_hollow_in_row(&self, row_index: u32) -> u32 {
        let mut result = 0;

        // Mix all color in single row to find the empty hole in row_index.
        for color in 0..self.board_data.board.len() {
            result = result | self.board_data.board[color][row_index as usize];
        }
        return result;
    }

    fn has_no_moves(&self) -> bool {
        for x in 0..self.board_data.width {
            for y in 0..self.board_data.height {
                if x < self.board_data.width - 1 {
                    // var result = this.simulate(new Move { x = x, y = y, direction = Direction.Right });
                    // if (result != null)
                    let mut next_board = self.clone();
                    let new_move = MoveAction {
                        x,
                        y,
                        direction: Direction::Right,
                    };

                    if next_board.do_bead_swap(&new_move).is_err() {
                        return true;
                    }

                    let clear_result = next_board.eval_clear_result(false);
                    if clear_result.is_some() {
                        return false;
                    }
                }

                if y < self.board_data.height - 1 {
                    // var result = this.simulate(new Move { x = x, y = y, direction = Direction.Up });
                    // if (result != null)
                    let mut next_board = self.clone();
                    let new_move = MoveAction {
                        x,
                        y,
                        direction: Direction::Up,
                    };

                    if next_board.do_bead_swap(&new_move).is_err() {
                        return true;
                    }

                    let clear_result = next_board.eval_clear_result(false);
                    if clear_result.is_some() {
                        return false;
                    }
                }
            }
        }
        return true;
    }

    fn eval_clear_result(&self, need_combo_result: bool) -> Option<(Vec<u32>, Vec<ComboState>)> {
        let num_matches = 3;
        let horizontal_match_mask = !((u32::MAX >> num_matches) << num_matches); //b'0000,0111'

        // Bit string of colors that are matched
        let mut colors_matched = 0;
        let width = self.board_data.width - 2 + num_matches;
        let mut board_clear_mask = vec![0; self.board_data.height as usize];
        let mut combo_states: Vec<ComboState> = vec![];

        // Repeat for each color
        for color_idx in 0..self.board_data.board.len() {
            let mut clear_mask = vec![0; self.board_data.height as usize];
            let current_board = &self.board_data.board[color_idx];
            // Vertical clears
            for row in (0..(self.board_data.height - num_matches + 1) as usize).rev() {
                let mut mask = self.board_field_mask; //b'0001,1111,1110'

                // Doing `&` operation downward 3 layer, if (mask != 0) there must exist at least 1 vertical match.
                for down_shift_count in 0..num_matches as usize {
                    let idx = row + down_shift_count;
                    mask = mask & (current_board[idx]);
                }
                // Merge the mask result to clear_mask
                if mask != 0 {
                    for down_shift_count in 0..num_matches as usize {
                        let idx = row + down_shift_count;
                        clear_mask[idx] = clear_mask[idx] | mask;
                    }
                    colors_matched = colors_matched | (1 << color_idx);
                }
            }

            // Horizontal clears
            for row in 0..self.board_data.height as usize {
                for left_shift_count in 0..width as usize {
                    let row_mask = horizontal_match_mask << left_shift_count;
                    if row_mask != (row_mask & current_board[row]) {
                        // no matches
                        continue;
                    }
                    // Merge the mask result to clear_mask
                    clear_mask[row] = clear_mask[row] | row_mask;
                    colors_matched = colors_matched | (1 << color_idx);
                }
            }

            if need_combo_result && colors_matched & (1 << color_idx) != 0 {
                combo_states.extend(self.eval_combo_states(color_idx, clear_mask.clone()));
            }

            // merge all color mask result
            for row in 0..self.board_data.height as usize {
                board_clear_mask[row] |= clear_mask[row];
            }
        }

        if need_combo_result {
            log::debug!("   Combo states: {:?}", combo_states);
        }

        if colors_matched != 0 {
            Some((board_clear_mask, combo_states))
        } else {
            None
        }
    }

    fn eval_combo_states(&self, color: usize, mut clear_mask_pattern: Vec<u32>) -> Vec<ComboState> {
        let mut combo_result: Vec<ComboState> = vec![];
        for column in 0..self.board_data.width + MASK_OFFSET {
            let column_mask = 1 << column;
            for row in 0..self.board_data.height as usize {
                // Find the mask position and count the amount by DFS
                if column_mask & clear_mask_pattern[row] != 0 {
                    let amount =
                        self.dfs_count_cluster(column_mask, row, &mut clear_mask_pattern, 0);
                    combo_result.push(ComboState {
                        color,
                        amount,
                        character_val_display: vec![], // Remain blank because character's attribute can't acquired at this moment.
                                                       // The actual value will be calculated later in game logic.
                    });
                }
            }
        }

        combo_result
    }

    fn dfs_count_cluster(
        &self,
        column_mask: u32,
        row: usize,
        clear_mask_pattern: &mut [u32],
        mut amount: u32,
    ) -> u32 {
        // Boundary check
        if (row == self.board_data.height as usize) || (column_mask & clear_mask_pattern[row] == 0)
        {
            return amount;
        }

        // Marked as visited
        clear_mask_pattern[row] &= !column_mask;
        amount += 1;

        //search left
        amount = self.dfs_count_cluster(
            column_mask << 1 & self.board_field_mask,
            row,
            clear_mask_pattern,
            amount,
        );

        //search right
        amount = self.dfs_count_cluster(
            column_mask >> 1 & self.board_field_mask,
            row,
            clear_mask_pattern,
            amount,
        );

        //search up
        if let Some(row_up) = row.checked_sub(1) {
            amount = self.dfs_count_cluster(column_mask, row_up, clear_mask_pattern, amount);
        }

        //search down
        amount = self.dfs_count_cluster(column_mask, row + 1, clear_mask_pattern, amount);

        return amount;
    }

    /// Do remove the beads in clear_mask
    fn apply_clear_mask(&mut self, board_clear_mask: Vec<u32>) {
        for c in 0..self.board_data.num_colors {
            for y in 0..self.board_data.height {
                let to_remove =
                    self.board_data.board[c as usize][y as usize] & board_clear_mask[y as usize];
                if to_remove != 0 {
                    let mut mask = 1 << MASK_OFFSET;
                    for _ in 0..self.board_data.width {
                        if (mask & to_remove) == mask {
                            self.board_data.remove_bead[c as usize] += 1;
                        }
                        mask = mask << 1;
                    }
                }
                self.board_data.board[c as usize][y as usize] =
                    self.board_data.board[c as usize][y as usize] & !board_clear_mask[y as usize];
            }
        }
    }

    fn process_falling_and_filling_result(&mut self, rng: &mut StdRng) -> Vec<BoardState> {
        let mut states = vec![];
        loop {
            match self.eval_clear_result(true) {
                None => break,
                Some((board_clear_mask, combo_states)) => {
                    states.push(BoardState::ClearState {
                        clear_mask: board_clear_mask.iter().map(|v| v >> MASK_OFFSET).collect(),
                        combo_states,
                    });
                    self.apply_clear_mask(board_clear_mask);
                    self.shift_empty();
                    self.refresh_reserved_block(self.board_data.height, rng);
                    states.push(BoardState::FillState {
                        board: self.clone(),
                    });

                    while self.has_no_moves() {
                        self.refresh_reserved_block(0, rng);
                        states.push(BoardState::RerollState {
                            board: self.clone(),
                        });
                    }
                }
            }
        }
        states
    }

    #[cfg(feature = "debug_tool")]
    pub fn replace_board(&mut self, import_board_data: &[[u32; 14]; 5]) -> Vec<BoardState> {
        let clear_mask = vec![std::u8::MAX as u32; 7];
        for color in 0..self.board_data.num_colors as usize {
            self.board_data.board[color] = import_board_data[color].to_vec();
        }

        // Replace board action implemented by mock Turntiles skill
        let mut states = Vec::new();
        states.push(BoardState::TurnTilesState {
            clear_mask,
            board: self.clone(),
        });

        states
    }
}

#[cfg(test)]
mod test {
    use super::{Board, BoardState, Direction, MoveAction};
    use rand::{rngs::StdRng, SeedableRng};
    use serde_json::Value;

    #[test]
    fn unit_test() {
        let mut rng = StdRng::seed_from_u64(777777u64);
        let mut init_board = Board::new(&mut rng, 5, 6, 6);
        let f_m = MoveAction {
            x: 0,
            y: 4,
            direction: Direction::Up,
        };
        let s = visualiztion_board(serde_json::json!(init_board.board_data.board));
        println!("{}", s);

        let state = init_board.simulate(&f_m, &mut rng).unwrap();
        for e in state {
            let s = match e {
                BoardState::ClearState {
                    clear_mask,
                    combo_states: _,
                } => visualiztion_mask(serde_json::json!(clear_mask)),
                BoardState::FillState { board } => {
                    visualiztion_board(serde_json::json!(board.board_data.board))
                }
                BoardState::RerollState { board } => {
                    visualiztion_board(serde_json::json!(board.board_data.board))
                }
                BoardState::TurnTilesState { clear_mask, board } => {
                    visualiztion_mask(serde_json::json!(clear_mask));
                    visualiztion_board(serde_json::json!(board.board_data.board))
                }
            };
            println!("{}", s);
        }
    }

    #[test]
    fn test() {
        let board_1 = serde_json::json!([
            [8, 10, 0, 10, 0, 8, 10, 66, 0, 32, 64, 16],
            [0, 64, 68, 4, 32, 86, 20, 20, 4, 4, 0, 0],
            [68, 0, 16, 16, 64, 0, 0, 40, 8, 10, 36, 46],
            [2, 52, 0, 64, 14, 0, 32, 0, 34, 80, 16, 0],
            [48, 0, 42, 32, 16, 32, 64, 0, 80, 0, 10, 64]
        ]);
        let mask_1 = serde_json::json!([0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let board_2 = serde_json::json!([
            [8, 10, 0, 10, 8, 10, 8, 0, 112, 32, 0, 72],
            [0, 64, 68, 4, 38, 84, 6, 32, 4, 0, 4, 34],
            [68, 0, 16, 16, 64, 0, 16, 18, 8, 66, 2, 0],
            [2, 52, 0, 64, 0, 0, 32, 76, 0, 0, 0, 16],
            [48, 0, 42, 32, 16, 32, 64, 0, 2, 28, 120, 4]
        ]);
        let mask_2 = serde_json::json!([0, 0, 2, 6, 6, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        println!("{}", visualiztion_board(board_1));
        println!("{}", visualiztion_mask(mask_1));
        println!("{}", visualiztion_board(board_2));
        println!("{}", visualiztion_mask(mask_2));
    }

    // debug helper
    pub fn visualiztion_board(board: Value) -> String {
        let height = 6;
        let width = 6;

        let mut result = String::from("=====board======\n");
        let board = serde_json::from_value::<Vec<Vec<u32>>>(board).unwrap();

        for i in (0..height).rev() {
            let blue_row = board[0][i].clone();
            let green_row = board[1][i].clone();
            let purple_row = board[2][i].clone();
            let red_row = board[3][i].clone();
            let orange_row = board[4][i].clone();

            for j in 1..(width + 1) {
                let mask = 1 << j;
                let char_to_add;

                if (blue_row & mask) != 0
                    && (green_row & mask) == 0
                    && (purple_row & mask) == 0
                    && (red_row & mask) == 0
                    && (orange_row & mask) == 0
                {
                    char_to_add = "b";
                } else if (green_row & mask) != 0
                    && (blue_row & mask) == 0
                    && (purple_row & mask) == 0
                    && (red_row & mask) == 0
                    && (orange_row & mask) == 0
                {
                    char_to_add = "g";
                } else if (purple_row & mask) != 0
                    && (blue_row & mask) == 0
                    && (green_row & mask) == 0
                    && (red_row & mask) == 0
                    && (orange_row & mask) == 0
                {
                    char_to_add = "p";
                } else if (red_row & mask) != 0
                    && (blue_row & mask) == 0
                    && (purple_row & mask) == 0
                    && (green_row & mask) == 0
                    && (orange_row & mask) == 0
                {
                    char_to_add = "r";
                } else if (orange_row & mask) != 0
                    && (blue_row & mask) == 0
                    && (purple_row & mask) == 0
                    && (green_row & mask) == 0
                    && (green_row & mask) == 0
                {
                    char_to_add = "o";
                } else {
                    char_to_add = "+";
                }
                result.push_str(char_to_add);
            }
            result.push_str("\n");
        }
        result.push_str("==========");

        result
    }

    pub fn visualiztion_mask(mask: Value) -> String {
        let mut result = String::from("=====clear_mask=====\n");
        let mask = serde_json::from_value::<Vec<u32>>(mask).unwrap();
        for i in (0..6).rev() {
            let mut row = mask[i];
            for _ in 0..6 {
                if (row & 1) == 1 {
                    result.push_str("x");
                } else {
                    result.push_str("+");
                }
                row = row >> 1;
            }
            result.push_str("\n");
        }
        result.push_str("==========");
        result
    }
}
