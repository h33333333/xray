#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ActionType {}

#[derive(Debug, Clone)]
pub enum Action {}

impl Action {
    pub fn get_action_type(&self) -> ActionType {
        todo!()
    }
}
