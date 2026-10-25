use crate::project::Project;

const LIMIT: usize = 200;

/// Snapshot undo. Projects hold no pixels, so whole-project snapshots are cheap,
/// and every editing surface gets undo for free without per-command code.
pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
    settled: Project,
}

impl History {
    pub fn new(project: &Project) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), settled: project.clone() }
    }

    pub fn reset(&mut self, project: &Project) {
        *self = Self::new(project);
    }

    /// Records a step once the user stops interacting, so a whole drag or slider
    /// gesture becomes a single undo step.
    pub fn observe(&mut self, current: &Project, interacting: bool) {
        if interacting || *current == self.settled {
            return;
        }
        let previous = std::mem::replace(&mut self.settled, current.clone());
        self.undo.push(previous);
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, current: &mut Project) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(current, prev.clone()));
        self.settled = prev;
        true
    }

    pub fn redo(&mut self, current: &mut Project) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(current, next.clone()));
        self.settled = next;
        true
    }
}
