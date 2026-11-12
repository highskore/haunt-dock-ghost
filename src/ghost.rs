//! Which face the Dock ghost shows. Free of host calls.

use crate::fleet::{Fleet, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Half-closed eyes: nothing running, nothing waiting.
    Sleepy,
    /// Open eyes and a small bob while agents work.
    Working,
    /// Wide awake, with how many agents need you.
    Awake(u32),
}

pub fn face(fleet: &Fleet) -> Face {
    match fleet.count(State::NeedsYou) {
        0 if fleet.count(State::Running) > 0 => Face::Working,
        0 => Face::Sleepy,
        n => Face::Awake(n.min(99) as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::Ev;

    #[test]
    fn faces_follow_the_fleet() {
        let mut f = Fleet::default();
        assert_eq!(face(&f), Face::Sleepy);
        f.apply(&Ev { surface: "a", kind: "prompt_submit", ..Ev::default() }, 1);
        assert_eq!(face(&f), Face::Working);
        f.apply(&Ev { surface: "b", kind: "permission_seen", ..Ev::default() }, 2);
        f.apply(&Ev { surface: "c", kind: "question_asked", ..Ev::default() }, 2);
        assert_eq!(face(&f), Face::Awake(2), "needing you wins over working");
        for s in ["a", "b", "c"] {
            f.apply(&Ev { surface: s, kind: "stop", ..Ev::default() }, 3);
        }
        assert_eq!(face(&f), Face::Sleepy);
    }
}
